#version 450
#pragma shader_stage(fragment)
#pragma optimize(on)

layout(location = 0) out vec4 fColor;

layout(set = 2, binding = 0) uniform sampler2D sTexture;

layout(location = 0) in vec2 UV;
layout(location = 1) in vec4 Color;

void main() { fColor = Color * texture(sTexture, UV.st); }