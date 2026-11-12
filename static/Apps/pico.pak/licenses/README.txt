lib/libSDL2-2.0.so.0 is an altered build of SDL2 (zlib, LICENSE.SDL2.txt) carrying
Steward Fu's Miyoo Mini drivers (LGPL-2.1, LICENSE.LGPL-2.1.txt) and stb_image
(MIT or public domain, LICENSE.stb_image.txt). Its source:

  https://github.com/mxmgorin/sdl2   branch: miyoomini-pico8   commit: 54318ffa

built with upstream's Docker recipe against mini_toolchain-v1.0: `make cfg && make sdl2`.
