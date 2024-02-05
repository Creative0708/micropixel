uniform sampler2D tex;

in mediump vec2 uv;
out lowp vec4 frag_color;

void main() {
    frag_color = texture(tex, uv);
}
