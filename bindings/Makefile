.DEFAULT_GOAL := all

LIBS := pywave/libwave_bindings.so pywave/libwave_bindings_debug.so

.PHONY: all
all: $(LIBS)

.PHONY: wheel
wheel: $(LIBS)
	python setup.py bdist_wheel

target/debug/libwaves_bindings.so:
	cargo build

pywave/libwave_bindings_debug.so: target/debug/libwave_bindings.so
	cp $< $@

pywave/libwave_bindings.so: target/release/libwave_bindings.so
	cp $< $@

-include target/debug/libwave_bindings.d
