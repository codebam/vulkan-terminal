#version 450

layout(location = 0) in vec2 inPosition;
layout(location = 1) in vec2 inTexCoord;
layout(location = 2) in vec4 inColor;

layout(push_constant) uniform PushConstants {
    vec2 screen_dimensions;
} push_constants;

layout(location = 0) out vec2 fragTexCoord;
layout(location = 1) out vec4 fragColor;

void main() {
    // Transform pixel coordinates to Normalized Device Coordinates (NDC)
    float ndc_x = (inPosition.x / push_constants.screen_dimensions.x) * 2.0 - 1.0;
    float ndc_y = (inPosition.y / push_constants.screen_dimensions.y) * 2.0 - 1.0;

    gl_Position = vec4(ndc_x, ndc_y, 0.0, 1.0);
    fragTexCoord = inTexCoord;
    fragColor = inColor;
}
