#version 450

layout(binding = 0) uniform sampler2D texSampler;

layout(location = 0) in vec2 fragTexCoord;
layout(location = 1) in vec4 fragColor;

layout(location = 0) out vec4 outColor;

void main() {
    // Sample the texture to get the alpha value of the glyph
    float alpha = texture(texSampler, fragTexCoord).r;
    
    // Combine the glyph's alpha with the desired text color
    outColor = vec4(fragColor.rgb, fragColor.a * alpha);
}