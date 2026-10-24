#version 450
#pragma shader_stage(vertex)
#pragma optimize(on)

layout(location = 0) in vec2 aPos;
layout(location = 1) in vec2 aUV;
layout(location = 2) in vec4 aColor;

layout(set = 1, binding = 0) uniform Projection { mat4 uProj; };

out gl_PerVertex { vec4 gl_Position; };

layout(location = 0) out vec2 UV;
layout(location = 1) out vec4 Color;

void main() {
  Color = aColor;
  UV = aUV;
  gl_Position = uProj * vec4(aPos, 0, 1);
}