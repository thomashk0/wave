import ctypes
import enum
import json
from ctypes import POINTER, byref
from typing import List, Optional, Tuple

import numpy as np
import pkg_resources

SO_PATH_DEBUG = pkg_resources.resource_filename('pywave',
                                                'libwave_bindings_debug.so')
SO_PATH = pkg_resources.resource_filename('pywave', 'libwave_bindings.so')

LIB = ctypes.cdll.LoadLibrary(SO_PATH)
LIB_DEBUG = ctypes.cdll.LoadLibrary(SO_PATH_DEBUG)


@enum.unique
class Status(enum.IntEnum):
    """Error codes for errors that may occur in Rust code
    """
    OK = 0
    IO_ERROR = 1
    PARSE_ERROR = 2
    MISSING_DATA = 3
    PARTIAL_HEADER = 4
    UTF8_ERROR = 5
    END_OF_INPUT = 6
    UNKNOWN = 255


class _StateSimS(ctypes.Structure):
    pass


for _lib in [LIB_DEBUG, LIB]:
    _lib.wave_sim_create.argtypes = (ctypes.c_char_p, POINTER(ctypes.c_int32),)
    _lib.wave_sim_create.restype = POINTER(_StateSimS)

    _lib.wave_sim_header_info.restype = ctypes.c_char_p
    _lib.wave_str_destroy.argtypes = (ctypes.c_char_p,)


def _raw_numpy_array(pointer, typestr, shape, copy=False, read_only_flag=False):
    buff = {'data': (pointer, read_only_flag),
            'typestr': typestr,
            'shape': shape}

    class NumpyHolder:
        pass

    holder = NumpyHolder()
    holder.__array_interface__ = buff
    return np.array(holder, copy=copy)


class WaveError(Exception):
    """Error that came from the Rust library"""

    def __init__(self, err: Status, message):
        super().__init__(message)
        self.err = err


class _ObjWrapper:
    """
    Wraps a Python dict object and give read access to its content as
    class attributes.
    """

    def __init__(self, obj, allowed_fields=None):
        self.obj = obj
        self.allowed_fields = allowed_fields or set()

    def __getattr__(self, item):
        if self.allowed_fields and item not in self.allowed_fields:
            raise AttributeError(f"no attribute named {item}")
        return self.obj[item]


class VariableInfo(_ObjWrapper):
    def __init__(self, obj, offset=None):
        super().__init__(obj)
        self.offset = offset

    def is_tracked(self) -> bool:
        return self.offset is not None

    def is_little_endian(self) -> bool:
        r = self.range
        if not isinstance(r, dict):
            return True
        rmin, rmax = tuple(r['Range'])
        return rmax > rmin

    @property
    def type(self):
        return self.obj['type']

    @property
    def scope(self):
        return ".".join(w['name'] for w in self.obj['scope'])

    def value(self, data: np.ndarray) -> int:
        s = self.offset
        c = data[s:s + self.width]
        if not np.all(c >= 0):
            raise ValueError("variable has bits in undefined state")
        order = "little" if self.is_little_endian() else "big"
        word = np.packbits(c, bitorder=order)
        if order == "big":
            word = word[::-1]
        return sum(int(x) << (8 * i) for i, x in enumerate(word))

    def __repr__(self):
        return f"VariableInfo<id='{self.id}', name={self.name}>"


class HeaderInfo(_ObjWrapper):
    def __init__(self, obj):
        super().__init__(obj)
        self.variables = {v[1]['id']: VariableInfo(v[1], offset=v[0]) for v in
                          self.obj.values()}

    @property
    def state_variables(self) -> List[VariableInfo]:
        """
        Returns the list of variables that appears in the state (were not
        excluded explicitly)
        """
        variables = [VariableInfo(x[1], offset=x[0]) for x in self.obj.values()
                     if x[0] is not None]
        variables.sort(key=lambda x: x.offset)
        return variables


class StateSim:
    def __init__(self, filename, lib=LIB):
        self.lib = lib
        status = ctypes.c_int32(0)
        self.handle = self.lib.wave_sim_create(filename.encode('utf-8'),
                                               ctypes.byref(status))
        if not self.handle:
            raise WaveError(Status(status.value),
                            "unable to create StateSim instance")

    def load_header(self):
        status = Status(self.lib.wave_sim_load_header(self.handle))
        if status != Status.OK:
            raise WaveError(status, "unable to load header")

    def allocate_state(self, restrict=None):
        p = None
        n = ctypes.c_size_t(0)
        if restrict:
            buff = [elt.encode('utf-8') for elt in restrict]
            p = (ctypes.c_char_p * len(buff))()
            p[:] = buff
            n = ctypes.c_size_t(len(buff))
        status = Status(self.lib.wave_sim_allocate_state(self.handle, p, n))
        if status != Status.OK:
            raise WaveError(status, "unable to allocate simulation state")

    def header_info(self) -> HeaderInfo:
        """Query waveform header information

        .. note:: Under the hoods, the header data (in Rust) are serialized as
         Python string and rebuilt on the Python side. This function in
         intended to be called only once per waveform.
        """
        s = None
        try:
            s = self.lib.wave_sim_header_info(self.handle)
            if not s:
                WaveError(Status.UNKNOWN, "unable to get header info")
            return HeaderInfo(json.loads(s))
        finally:
            self.lib.wave_str_destroy(s)

    def next_cycle(self) -> Optional[Tuple[int, np.ndarray]]:
        """
        Runs the parser until the end of the next simulation cycle (or
        the end of the file)

        .. warning::

            For efficiency reasons, the numpy array returned is a direct view
            on memory, hence it will be modified on the next call of the
            :py:method:`next_cycle` method. You must copy the array explicitly
            if you do not need this behavior.

        :return: None if the simulation is done (no more things to parse),
                 otherwise returns the cycle and the value of all signals
                 requested as a numpy array of int8.
        """
        cycle = ctypes.c_int64(0)
        size = ctypes.c_uint64(0)
        p = ctypes.c_void_p()
        status = Status(
            self.lib.wave_sim_next_cycle(self.handle, byref(cycle), byref(p),
                                         byref(size)))
        if status == Status.END_OF_INPUT:
            return None
        if status != Status.OK:
            raise WaveError(status, "failed to get state for next cycle")

        t = _raw_numpy_array(p.value, "<i1", (int(size.value),))
        return cycle.value, t

    def __del__(self):
        self.lib.wave_sim_destroy(self.handle)
        self.handle = None


BIT_REPR = {0: '0', 1: '1', -1: 'U', -2: 'W', -3: 'Z', -4: 'X'}
