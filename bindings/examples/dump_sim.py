"""
This small example demonstrate the use of the pywave module to dump all signals
values from a waveform file (eg., in VCD format)
"""
import argparse
import sys

from pywave import BIT_REPR, StateSim, WaveError


def dump_signals(variables, data):
    for v in variables:
        s = v.offset
        print(f"{v.id:<2}: {v.name:20} ->",
              "".join(BIT_REPR[x] for x in data[s:s + v.width]))


def main():
    parser = argparse.ArgumentParser(description="")
    parser.add_argument('-n', '--num-cycles', default=10,
                        help='Number of cycles to dump')
    parser.add_argument('input', metavar="file.vcd", help='Input file')
    args = parser.parse_args()

    try:
        sim = StateSim(args.input)
        sim.load_header()
        info = sim.header_info()
        for i in range(args.num_cycles):
            s = sim.next_cycle()
            if not s:
                print(f"info: simulation stopped after {i} iterations")
                break
            c, data = s
            if c == -1:
                print(f"== Initial State (cycle = -1)")
            else:
                print()
                print(f"== Cycle {c}")
            dump_signals(info.variables, data)
    except WaveError as e:
        print(f"error: something went wrong in FFI layer -> {str(e.err)} ({e})")
        sys.exit(1)


if __name__ == '__main__':
    main()
