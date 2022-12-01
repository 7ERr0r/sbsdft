# This Makefile generates SPIR-V shaders from GLSL shaders in the examples.

shader_compiler = ./bin/glslangValidator.exe

# All input shaders.
glsls = $(wildcard src/shaders/*.vert src/shaders/*.frag src/shaders/*.comp)

# All SPIR-V targets.
spirvs = $(addsuffix .spv,$(glsls))


.PHONY: default
default: $(spirvs)


# Rule for making a SPIR-V target.
$(spirvs): %.spv: %
	@echo $(spirvs)
	$(shader_compiler) -V $< -o $@

.PHONY: clean
clean:
	rm -f $(spirvs)