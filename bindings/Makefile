.DEFAULT_GOAL := all

HERE := $(shell pwd)
LIBS := libwavetk_bindings.so libwavetk_bindings_debug.so
LIBS_OUT := $(addprefix wavetk/,$(LIBS))

.PHONY: all
all: $(LIBS_OUT)

.PHONY: wheel
wheel: $(LIBS_OUT)
	python setup.py bdist_wheel

.PHONY: clean
clean:
	-rm -f $(LIBS_OUT)

$(HERE)/target/debug/libwavetk_bindings.so:
	cargo build

$(HERE)/target/release/libwavetk_bindings.so:
	cargo build --release

wavetk/libwavetk_bindings_debug.so: $(HERE)/target/debug/libwavetk_bindings.so
	cp $< $@

wavetk/libwavetk_bindings.so: $(HERE)/target/release/libwavetk_bindings.so
	cp $< $@

-include target/debug/libwavetk_bindings.d
-include target/release/libwavetk_bindings.d
