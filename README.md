# Path Tracer
A fun little path tracer

## To build & run
Dependencies:
- SDL3
- Eigen
```
cmake -S . -B build
cmake --build build
.\build\pathtrace.exe
```

## TODO
- [ ] Different materials
    - [ ] Metallic
    - [ ] Glossy
    - [ ] Subsurface scatter
- [ ] Triangles and quads, loading .obj files
- [ ] BVH
- [ ] Textures
- [ ] PBR
- [ ] Adaptive sampling, denoising
- [ ] Lens aperture
- [ ] True spectral rendering (rainbows!)
- [ ] Volumetrics