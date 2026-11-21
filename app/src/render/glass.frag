#version 100
// Human: Quill's liquid glass material. A slab of glass with a curved rim lies on what is behind the
// layer: the rim bends that picture inward (Snell's law on a squircle-profiled bevel), light catches the rim
// from one side and, weaker, from the opposite one, and a tint lies over it all. Written from the optics, as
// an original material; it follows the published look of such glass, not anyone's code.
// Agent: runs through `shader::compile` like a shader effect; `gl_FragCoord` is the offset from the layer's
// centre in document units (y up), and `u_backdrop` is read at an offset from the centre in document units
// (y down). Colours are premultiplied; the maths is in linear light and the result goes out gamma-encoded.
precision highp float;

/** What is behind the layer, already blurred in linear light; gamma-encoded, premultiplied. */
uniform sampler2D u_backdrop;
/** The same, only lightly blurred: what the rim shows, so that its bending can be seen. */
uniform sampler2D u_rim;
/** How many outlines the pane joins, 1 to 8. */
uniform float u_count;
/** Outlines closer than this flow into each other. */
uniform float u_merge;
/** Outline 0: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box0;
/** Outline 0's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round0;
/** Outline 1: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box1;
/** Outline 1's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round1;
/** Outline 2: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box2;
/** Outline 2's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round2;
/** Outline 3: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box3;
/** Outline 3's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round3;
/** Outline 4: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box4;
/** Outline 4's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round4;
/** Outline 5: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box5;
/** Outline 5's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round5;
/** Outline 6: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box6;
/** Outline 6's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round6;
/** Outline 7: its centre's offset from the pane's centre, and half its size. */
uniform vec4 u_box7;
/** Outline 7's corner radii (top left, top right, bottom right, bottom left); a first one below 0 is an ellipse. */
uniform vec4 u_round7;
/** Turns a document offset into the layer's own: x' = a x + c y, y' = b x + d y. */
uniform vec4 u_turn;
/** Pixels per document unit. */
uniform float u_scale;
/** Width of the curved rim. */
uniform float u_bevel;
/** Thickness of the glass. */
uniform float u_thickness;
/** How strongly the rim bends; 1 is glass. */
uniform float u_refraction;
/** Colour split at the rim, 0-1. */
uniform float u_dispersion;
/** Colour strength of what is behind. */
uniform float u_saturation;
/** Added to what is behind. */
uniform float u_brightness;
/** The tint in linear light, and how much of it. */
uniform vec4 u_tint;
/** Strength of the light on the rim. */
uniform float u_highlight;
/** Unit vector toward the light, document space (y down). */
uniform vec2 u_light;

const float IOR = 1.5;
/** The steepest the rim's profile is taken to be; it is upright at the very edge. */
const float STEEPEST = 6.0;
/** How much of the glass's thickness lies under the rim's outer edge. */
const float BASE = 0.25;
/** The width of the bright hairline along the edge, in document units. */
const float HAIRLINE = 1.1;
/** How much of the rim's picture is the lightly blurred one, at the very edge. */
const float SHARP_RIM = 0.9;

vec3 to_linear(vec3 c) {
    return mix(c / 12.92, pow((c + 0.055) / 1.055, vec3(2.4)), step(0.04045, c));
}

vec3 to_gamma(vec3 c) {
    c = clamp(c, 0.0, 1.0);
    return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(0.0031308, c));
}

float box_distance(vec2 p, vec2 half_size, vec4 radii) {
    float r = p.x < 0.0 ? (p.y < 0.0 ? radii.x : radii.w) : (p.y < 0.0 ? radii.y : radii.z);
    vec2 q = abs(p) - half_size + r;
    return min(max(q.x, q.y), 0.0) + length(max(q, 0.0)) - r;
}

float oval_distance(vec2 p, vec2 half_size) {
    float k0 = length(p / half_size);
    float k1 = length(p / (half_size * half_size));
    return k1 > 1e-6 ? k0 * (k0 - 1.0) / k1 : -min(half_size.x, half_size.y);
}

float shape_distance(vec2 p, vec4 box, vec4 radii) {
    return radii.x < 0.0 ? oval_distance(p - box.xy, box.zw) : box_distance(p - box.xy, box.zw, radii);
}

/** The smaller of two distances, rounded off where they are within `k` of each other. */
float smooth_min(float a, float b, float k) {
    if (k <= 1e-3) {
        return min(a, b);
    }
    float h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

/** The distance from `p` (the pane's own space) to the pane's edge; negative inside. */
float edge_distance(vec2 p) {
    float d = shape_distance(p, u_box0, u_round0);
    if (u_count > 0.5) d = smooth_min(d, shape_distance(p, u_box1, u_round1), u_merge);
    if (u_count > 1.5) d = smooth_min(d, shape_distance(p, u_box2, u_round2), u_merge);
    if (u_count > 2.5) d = smooth_min(d, shape_distance(p, u_box3, u_round3), u_merge);
    if (u_count > 3.5) d = smooth_min(d, shape_distance(p, u_box4, u_round4), u_merge);
    if (u_count > 4.5) d = smooth_min(d, shape_distance(p, u_box5, u_round5), u_merge);
    if (u_count > 5.5) d = smooth_min(d, shape_distance(p, u_box6, u_round6), u_merge);
    if (u_count > 6.5) d = smooth_min(d, shape_distance(p, u_box7, u_round7), u_merge);
    return d;
}

/** What is behind the layer at a document offset from its centre, in linear light. */
vec4 behind(vec2 at, float sharp) {
    vec4 c = mix(texture2D(u_backdrop, at), texture2D(u_rim, at), sharp);
    return vec4(to_linear(c.rgb / max(c.a, 1e-4)) * c.a, c.a);
}

/** How far sideways, per unit of depth, a ray straight down travels after entering glass of index `ior`
    through a surface of `slope`: tan(incidence - refraction). */
float bend(float slope, float ior) {
    float sin_i = slope / sqrt(1.0 + slope * slope);
    float sin_t = sin_i / ior;
    float tan_t = sin_t / sqrt(max(1.0 - sin_t * sin_t, 1e-4));
    return (slope - tan_t) / (1.0 + slope * tan_t);
}

void main() {
    vec2 offset = vec2(gl_FragCoord.x, -gl_FragCoord.y);
    vec2 p = vec2(u_turn.x * offset.x + u_turn.z * offset.y, u_turn.y * offset.x + u_turn.w * offset.y);
    float dist = edge_distance(p);
    float cover = clamp(0.5 - dist * u_scale, 0.0, 1.0);
    if (cover <= 0.0) {
        gl_FragColor = vec4(0.0);
        return;
    }
    // The way out of the shape, from the distance's slope, turned back into document space.
    float e = max(0.5 / u_scale, 0.02);
    vec2 slope_of = vec2(
        edge_distance(p + vec2(e, 0.0)) - edge_distance(p - vec2(e, 0.0)),
        edge_distance(p + vec2(0.0, e)) - edge_distance(p - vec2(0.0, e)));
    float steep = length(slope_of);
    vec2 out_layer = steep > 1e-5 ? slope_of / steep : vec2(0.0);
    vec2 outward = vec2(u_turn.x * out_layer.x + u_turn.y * out_layer.y, u_turn.z * out_layer.x + u_turn.w * out_layer.y);

    // The rim's profile: a squircle, flat where it meets the top of the slab.
    float inside = max(-dist, 0.0);
    float x = u_bevel > 1e-3 ? clamp(inside / u_bevel, 0.0, 1.0) : 1.0;
    float k = 1.0 - x;
    float k4 = k * k * k * k;
    float height = pow(max(1.0 - k4, 0.0), 0.25);
    float slope = u_bevel > 1e-3 ? (u_thickness / u_bevel) * k * k * k / pow(max(1.0 - k4, 1e-4), 0.75) : 0.0;
    slope = min(slope, STEEPEST);
    float depth = (height * (1.0 - BASE) + BASE) * u_thickness * u_refraction;

    // The rim is the clearest part of the glass: frost lies on the flat top, and thins out over the bevel.
    float sharp = SHARP_RIM * k * sqrt(k);
    vec4 glass;
    if (u_dispersion > 1e-3) {
        float split = 0.12 * u_dispersion;
        vec4 red = behind(offset - outward * bend(slope, IOR - split) * depth, sharp);
        vec4 green = behind(offset - outward * bend(slope, IOR) * depth, sharp);
        vec4 blue = behind(offset - outward * bend(slope, IOR + split) * depth, sharp);
        glass = vec4(red.r, green.g, blue.b, green.a);
    } else {
        glass = behind(offset - outward * bend(slope, IOR) * depth, sharp);
    }

    // Frost: stronger colour, a little light, then the tint over it.
    float grey = dot(glass.rgb, vec3(0.2126, 0.7152, 0.0722));
    glass.rgb = max(mix(vec3(grey), glass.rgb, u_saturation), 0.0);
    glass.rgb = clamp(glass.rgb + u_brightness * glass.a, 0.0, 1.0);
    glass = vec4(mix(glass.rgb, u_tint.rgb, u_tint.a), mix(glass.a, 1.0, u_tint.a));

    // Light on the rim: from the light's side, weaker from the opposite side, and brightest where the
    // surface turns away from the eye (Fresnel).
    float facing = dot(outward, u_light);
    float key = max(facing, 0.0);
    float fill = max(-facing, 0.0);
    float lit = key * key + 0.45 * fill * fill;
    float up = 1.0 / sqrt(1.0 + slope * slope);
    float fresnel = pow(1.0 - up, 3.0);
    float hairline = 1.0 - smoothstep(0.0, HAIRLINE, inside);
    float band = k * k;
    float light = u_highlight * (hairline * (0.16 + 0.84 * lit) + band * (0.10 * fresnel + 0.22 * lit * fresnel));
    // Thick glass is a little darker inside its rim on the side away from the light.
    float shade = 0.10 * u_highlight * band * (1.0 - hairline) * fill;
    glass.rgb *= 1.0 - shade;
    glass = vec4(glass.rgb + light * (1.0 - glass.rgb), glass.a + light * (1.0 - glass.a));

    // Out: gamma-encoded, with a little noise so soft gradients don't band, premultiplied by the edge.
    vec3 straight = to_gamma(glass.rgb / max(glass.a, 1e-4));
    vec2 grain = offset * u_scale;
    float noise = fract(sin(dot(grain, vec2(12.9898, 78.233))) * 43758.5453) - 0.5;
    straight = clamp(straight + noise / 255.0, 0.0, 1.0);
    float alpha = glass.a * cover;
    gl_FragColor = vec4(straight * alpha, alpha);
}
