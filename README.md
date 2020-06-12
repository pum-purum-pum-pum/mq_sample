# Miniquad sample

[Demo](https://pum-purum-pum-pum.github.io/mq_sample/)

It uses miniquad and some ancient opengl shaders.

This sample consists of:
* Deformed texture rendering using projection textures
* Shadows using offscreen pipeline and trick with projection textures (not fair shadows)
* Simple triangle antialiasing using signed distance filed


Known issues and possible enhancements:
* raycasting inside the polygon
* shadow glitch when blocking segment changes -- shadows in this demo are not fair shadows :)
* aliased polygons