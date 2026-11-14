//! Shader files: reading a WebGL 1.0 fragment shader, its uniforms and their `@directive` notes, and turning
//! it into the GLSL the GPU library reads.
//!
//! A shader file is written for WebGL (`#version 100`): loose `uniform`s, `gl_FragColor`, `texture2D`. wgpu's
//! GLSL reader takes Vulkan-style GLSL 450 only: uniforms in one block, textures apart from their samplers, a
//! declared output. [`compile`] rewrites the one into the other around the untouched body of the shader, and
//! checks the result, so a shader that can't run is known before the GPU sees it.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::SystemTime;

use pen_doc::color::Rgba;
use serde_json::Value;
use wgpu::naga;

/// The uniform block every converted shader starts with.
const BLOCK: &str = "PcParams";
/// The block member that places the shader on its picture: the node's left edge and bottom edge in pixels, and
/// the pixels per unit.
pub const FRAME: &str = "pc_frame";

/// The type of a uniform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// `float`.
    Float,
    /// `int`.
    Int,
    /// `bool`.
    Bool,
    /// `vec2`, `vec3` or `vec4`.
    Vec(usize),
    /// `ivec2`, `ivec3` or `ivec4`.
    IVec(usize),
    /// `sampler2D`.
    Sampler,
}

impl Type {
    fn of(word: &str) -> Option<Type> {
        Some(match word {
            "float" => Type::Float,
            "int" => Type::Int,
            "bool" => Type::Bool,
            "vec2" => Type::Vec(2),
            "vec3" => Type::Vec(3),
            "vec4" => Type::Vec(4),
            "ivec2" => Type::IVec(2),
            "ivec3" => Type::IVec(3),
            "ivec4" => Type::IVec(4),
            "sampler2D" => Type::Sampler,
            _ => return None,
        })
    }

    /// How many numbers a value of this type has.
    #[must_use]
    pub fn len(self) -> usize {
        match self {
            Type::Float | Type::Int | Type::Bool => 1,
            Type::Vec(n) | Type::IVec(n) => n,
            Type::Sampler => 0,
        }
    }

    /// Whether its numbers are whole (`int`, `bool`, `ivec`).
    fn whole(self) -> bool {
        matches!(self, Type::Int | Type::Bool | Type::IVec(_))
    }
}

/// What the app sets a uniform to by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Nothing: the value comes from the layer, or the shader's `@default`.
    Plain,
    /// `@resolution`: the node's size.
    Resolution,
    /// `@time`: seconds since the start. A still picture has 0.
    Time,
    /// `@mouse`: the pointer's position. Not followed in this version.
    Mouse,
    /// `@sdf`: the distance to the shape's edge. Not worked out in this version.
    Sdf,
}

/// One uniform of a shader with what its comment says about it.
#[derive(Clone, Debug, PartialEq)]
pub struct Uniform {
    /// Its name in the shader, and the key of its value in the layer's `uniforms`.
    pub name: String,
    /// Its type.
    pub ty: Type,
    /// What the app sets it to by itself.
    pub role: Role,
    /// `@label`: its name in the interface.
    pub label: Option<String>,
    /// `@default`: its value while the layer sets none.
    pub default: Option<String>,
    /// `@color`: a `vec3` or `vec4` shown as a colour.
    pub color: bool,
    /// `@range`, or `@min` and `@max`.
    pub range: (Option<f32>, Option<f32>),
}

impl Uniform {
    /// The name to show for it.
    #[must_use]
    pub fn title(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.name)
    }

    /// Whether the user gives it a value (the app sets the others).
    #[must_use]
    pub fn editable(&self) -> bool {
        self.role == Role::Plain
    }
}

/// Where a `sampler2D` gets its picture.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// `pencil:object`: the layer as drawn so far.
    Object,
    /// `pencil:background`: what is behind the layer.
    Background,
    /// A picture file.
    File(String),
    /// Nothing; samples as clear.
    Nothing,
}

/// A shader ready for the GPU.
#[derive(Debug)]
pub struct Program {
    /// Its uniforms, in the order the file declares them.
    pub uniforms: Vec<Uniform>,
    /// The converted source.
    pub glsl: String,
    /// The size of its uniform block in bytes.
    pub block_size: usize,
    /// Where each member of the block starts.
    offsets: HashMap<String, usize>,
}

/// A shader file as last read: when it was changed, and what reading it gave.
type Loaded = (Option<SystemTime>, Result<Arc<Program>, String>);

/// The shader in the file at `path`, read and compiled again only when the file has changed.
///
/// # Errors
/// A sentence when the file can't be read or the shader can't run.
// Agent: LOCKS the shader store briefly; READS the file's change time on every call and its text when that moved.
pub fn load(path: &Path) -> Result<Arc<Program>, String> {
    static STORE: OnceLock<Mutex<HashMap<PathBuf, Loaded>>> = OnceLock::new();
    let changed = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut store = crate::app::lock(STORE.get_or_init(Mutex::default));
    if let Some((seen, result)) = store.get(path)
        && *seen == changed
    {
        return result.clone();
    }
    let result = std::fs::read_to_string(path).map_err(|e| format!("Couldn't read the file: {e}")).and_then(|source| compile(&source)).map(Arc::new);
    store.insert(path.to_path_buf(), (changed, result.clone()));
    result
}

/// The block member that maps a sampler's texture coordinates onto its picture: scale x, y, then offset x, y.
#[must_use]
pub fn map_member(sampler: &str) -> String {
    format!("pc_map_{sampler}")
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// A `uniform` declaration found in a shader: the bytes it spans, and what it declares.
struct Declared {
    span: (usize, usize),
    uniforms: Vec<Uniform>,
}

/// The `@directive`s of a comment, each with the rest of its line.
fn directives(comment: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for line in comment.lines() {
        let mut rest = line;
        while let Some(at) = rest.find('@') {
            let after = &rest[at + 1..];
            let end = after.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(after.len());
            let (name, tail) = after.split_at(end);
            // Human: a directive's value runs to the next directive on the line, or the line's end.
            let value = tail.split('@').next().unwrap_or("").trim().trim_end_matches("*/").trim();
            if !name.is_empty() {
                found.push((name.to_ascii_lowercase(), value.to_owned()));
            }
            rest = tail;
        }
    }
    found
}

/// Every `uniform` declaration of `source`, each with the block comment right above it.
// Human: the shader format puts a uniform's notes in the `/** … */` comment before it.
fn declarations(source: &str) -> Result<Vec<Declared>, String> {
    let bytes = source.as_bytes();
    let mut found = Vec::new();
    let mut comment: Option<&str> = None;
    let mut i = 0;
    while i < bytes.len() {
        let rest = &source[i..];
        if rest.starts_with("//") {
            i += rest.find('\n').unwrap_or(rest.len());
        } else if rest.starts_with("/*") {
            let end = rest.find("*/").map_or(rest.len(), |e| e + 2);
            comment = Some(&rest[..end]);
            i += end;
        } else if bytes[i].is_ascii_whitespace() {
            i += 1;
        } else if is_word(bytes[i]) {
            let end = rest.bytes().position(|b| !is_word(b)).unwrap_or(rest.len());
            if &rest[..end] == "uniform" {
                let stop = rest.find(';').ok_or("a uniform declaration has no ending semicolon")?;
                let mut words =
                    rest[end..stop].split([' ', '\t', '\r', '\n', ',']).filter(|w| !w.is_empty() && !matches!(*w, "lowp" | "mediump" | "highp"));
                let type_name = words.next().ok_or("a uniform declaration has no type")?;
                let ty = Type::of(type_name).ok_or_else(|| format!("uniforms of type `{type_name}` aren't supported"))?;
                let notes = comment.map(directives).unwrap_or_default();
                let uniforms = words.map(|name| uniform(name, ty, &notes)).collect::<Result<Vec<_>, _>>()?;
                found.push(Declared { span: (i, i + stop + 1), uniforms });
                i += stop + 1;
            } else {
                i += end;
            }
            comment = None;
        } else {
            comment = None;
            i += rest.chars().next().map_or(1, char::len_utf8);
        }
    }
    Ok(found)
}

/// A uniform from its name, type and comment notes.
fn uniform(name: &str, ty: Type, notes: &[(String, String)]) -> Result<Uniform, String> {
    if !name.bytes().all(is_word) {
        return Err(format!("the uniform `{name}` isn't supported (arrays and initial values aren't)"));
    }
    let has = |key: &str| notes.iter().any(|(name, _)| name == key);
    let text = |key: &str| notes.iter().find(|(name, _)| name == key).map(|(_, value)| value.clone()).filter(|v| !v.is_empty());
    let number = |key: &str| text(key).and_then(|v| v.parse::<f32>().ok());
    let range: Vec<f32> = text("range").map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect()).unwrap_or_default();
    let role = match ty {
        Type::Sampler if has("sdf") => Role::Sdf,
        Type::Sampler => Role::Plain,
        _ if has("resolution") => Role::Resolution,
        _ if has("time") => Role::Time,
        _ if has("mouse") => Role::Mouse,
        _ => Role::Plain,
    };
    Ok(Uniform {
        name: name.to_owned(),
        ty,
        role,
        label: text("label"),
        default: text("default"),
        color: has("color") && matches!(ty, Type::Vec(3 | 4)),
        range: (range.first().copied().or_else(|| number("min")), range.get(1).copied().or_else(|| number("max"))),
    })
}

/// `source` with every call `function(sampler, …` turned into `replacement(…`.
fn redirect_calls(source: &str, function: &str, sampler: &str, replacement: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while let Some(found) = source[i..].find(function) {
        let start = i + found;
        let after = start + function.len();
        let whole = (start == 0 || !is_word(bytes[start - 1])) && bytes.get(after).is_none_or(|b| !is_word(*b));
        let spaces = |at: usize| at + source[at..].bytes().take_while(u8::is_ascii_whitespace).count();
        let open = spaces(after);
        let name = if bytes.get(open) == Some(&b'(') { spaces(open + 1) } else { usize::MAX };
        let named = whole && name != usize::MAX && source[name..].starts_with(sampler);
        let comma = if named { spaces(name + sampler.len()) } else { usize::MAX };
        if named && bytes.get(comma) == Some(&b',') {
            out.push_str(&source[i..start]);
            out.push_str(replacement);
            out.push('(');
            i = comma + 1;
        } else {
            out.push_str(&source[i..after]);
            i = after;
        }
    }
    out.push_str(&source[i..]);
    out
}

/// The GLSL type name of a block member.
fn member_type(ty: Type) -> String {
    match ty {
        Type::Float => "float".to_owned(),
        // Human: a uniform block can't hold a `bool`; it travels as an `int` and a macro turns it back.
        Type::Int | Type::Bool => "int".to_owned(),
        Type::Vec(n) => format!("vec{n}"),
        Type::IVec(n) => format!("ivec{n}"),
        Type::Sampler => String::new(),
    }
}

/// The shader as GLSL 450: the uniforms gathered in a block, each sampler split into a texture, a sampler and
/// functions to read it, and the shader's `main` called from a new one that sets up `gl_FragCoord`.
// Human: `gl_FragCoord` is in the node's own units with y going up from its bottom edge, as in WebGL, and a
// sampler's texture coordinates have v = 0 at the bottom; the wrapper and the `pc_map_…` members do both flips.
// Agent: WRITES to a String only, which can't fail; every name it adds starts with `pc_`.
fn convert(source: &str, declared: &[Declared]) -> String {
    let mut body = source.to_owned();
    // Blank out the uniform declarations (they move into the block), keeping the line numbers.
    for declaration in declared.iter().rev() {
        let (start, end) = declaration.span;
        let blank: String = source[start..end].chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect();
        body.replace_range(start..end, &blank);
    }
    let body: String = body
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            let dropped =
                trimmed.starts_with("#version") || trimmed.starts_with("#extension") || (trimmed.starts_with("precision ") && trimmed.contains(';'));
            if dropped { "" } else { line }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let uniforms: Vec<&Uniform> = declared.iter().flat_map(|d| &d.uniforms).collect();
    let samplers: Vec<&Uniform> = uniforms.iter().copied().filter(|u| u.ty == Type::Sampler).collect();
    let mut body = body;
    for sampler in &samplers {
        body = redirect_calls(&body, "texture2D", &sampler.name, &format!("pc_sample_{}", sampler.name));
        body = redirect_calls(&body, "textureSize", &sampler.name, &format!("pc_size_{}", sampler.name));
    }

    let mut out = format!("#version 450\nlayout(set = 0, binding = 0) uniform {BLOCK} {{\n    vec4 {FRAME};\n");
    for sampler in &samplers {
        let _ = writeln!(out, "    vec4 {};", map_member(&sampler.name));
    }
    for uniform in uniforms.iter().filter(|u| u.ty != Type::Sampler) {
        let prefix = if uniform.ty == Type::Bool { "pc_bool_" } else { "" };
        let _ = writeln!(out, "    {} {prefix}{};", member_type(uniform.ty), uniform.name);
    }
    out.push_str("};\n");
    for uniform in uniforms.iter().filter(|u| u.ty == Type::Bool) {
        let _ = writeln!(out, "#define {0} (pc_bool_{0} != 0)", uniform.name);
    }
    for (index, sampler) in samplers.iter().enumerate() {
        let (name, map) = (&sampler.name, map_member(&sampler.name));
        let pair = format!("sampler2D(pc_tex_{name}, pc_smp_{name})");
        let _ = writeln!(out, "layout(set = 0, binding = {}) uniform texture2D pc_tex_{name};", 1 + 2 * index);
        let _ = writeln!(out, "layout(set = 0, binding = {}) uniform sampler pc_smp_{name};", 2 + 2 * index);
        // Human: level 0 is read outright, so a shader may sample inside an `if` or a loop.
        let _ = writeln!(out, "vec4 pc_sample_{name}(vec2 uv) {{ return textureLod({pair}, uv * {map}.xy + {map}.zw, 0.0); }}");
        let _ = writeln!(out, "ivec2 pc_size_{name}(int lod) {{ return textureSize({pair}, lod); }}");
    }
    out.push_str("layout(location = 0) out vec4 pc_color;\nvec4 pc_coord;\n");
    out.push_str("#define gl_FragColor pc_color\n#define gl_FragCoord pc_coord\n#define main pc_main\n");
    out.push_str(&body);
    out.push_str("\n#undef main\n#undef gl_FragCoord\nvoid main() {\n");
    let _ = writeln!(out, "    pc_coord = vec4((gl_FragCoord.x - {FRAME}.x) / {FRAME}.z, ({FRAME}.y - gl_FragCoord.y) / {FRAME}.z, 0.0, 1.0);");
    out.push_str("    pc_color = vec4(0.0);\n    pc_main();\n}\n");
    out
}

/// Reads a shader file's source: its uniforms, and the source converted and checked.
///
/// # Errors
/// A sentence saying what is wrong, when the shader uses something this version can't run or doesn't compile.
pub fn compile(source: &str) -> Result<Program, String> {
    let declared = declarations(source)?;
    let glsl = convert(source, &declared);
    let module = naga::front::glsl::Frontend::default()
        .parse(&naga::front::glsl::Options::from(naga::ShaderStage::Fragment), &glsl)
        .map_err(|errors| errors.errors.iter().map(|e| e.kind.to_string()).collect::<Vec<_>>().join("; "))?;
    naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty())
        .validate(&module)
        .map_err(|error| error.into_inner().to_string())?;
    // Human: where each uniform sits in the block is read back from the compiled shader, not worked out a
    // second time here, so the two can't disagree.
    let (members, span) = module
        .types
        .iter()
        .find_map(|(_, ty)| match (&ty.name, &ty.inner) {
            (Some(name), naga::TypeInner::Struct { members, span }) if name == BLOCK => Some((members, *span)),
            _ => None,
        })
        .ok_or("the shader's uniform block went missing")?;
    let offsets = members
        .iter()
        .filter_map(|member| Some((member.name.clone()?.trim_start_matches("pc_bool_").to_owned(), usize::try_from(member.offset).ok()?)))
        .collect();
    let block_size = usize::try_from(span).map_err(|_| "the shader's uniform block is too large")?.next_multiple_of(16);
    Ok(Program { uniforms: declared.into_iter().flat_map(|d| d.uniforms).collect(), glsl, block_size, offsets })
}

impl Program {
    /// The bytes of the uniform block: `frame` and the samplers' maps, then each uniform's numbers from `value`.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "a whole-number uniform; `as` saturates at the i32 range")]
    pub fn block(&self, frame: [f32; 4], maps: &[(String, [f32; 4])], value: impl Fn(&Uniform) -> [f32; 4]) -> Vec<u8> {
        let mut bytes = vec![0_u8; self.block_size];
        let mut put = |member: &str, index: usize, raw: [u8; 4]| {
            if let Some(slot) = self.offsets.get(member).and_then(|offset| bytes.get_mut(offset + 4 * index..offset + 4 * index + 4)) {
                slot.copy_from_slice(&raw);
            }
        };
        for (member, numbers) in std::iter::once((FRAME, frame)).chain(maps.iter().map(|(name, map)| (name.as_str(), *map))) {
            for (index, number) in numbers.iter().enumerate() {
                put(member, index, number.to_ne_bytes());
            }
        }
        for uniform in self.uniforms.iter().filter(|u| u.ty != Type::Sampler) {
            let numbers = value(uniform);
            for (index, number) in numbers.iter().take(uniform.ty.len()).enumerate() {
                put(&uniform.name, index, if uniform.ty.whole() { (number.round() as i32).to_ne_bytes() } else { number.to_ne_bytes() });
            }
        }
        bytes
    }
}

/// Up to four numbers from a piece of text: a hex colour, `true` or `false`, or numbers however they are set
/// apart (`1, 2`, `vec2(1.0, 2.0)`).
#[must_use]
pub fn numbers_in(text: &str) -> Option<[f32; 4]> {
    let text = text.trim();
    if text.starts_with('#') {
        let c = Rgba::parse(text)?;
        return Some([c.r, c.g, c.b, c.a].map(|v| f32::from(v) / 255.0));
    }
    match text {
        "true" => return Some([1.0, 0.0, 0.0, 0.0]),
        "false" => return Some([0.0; 4]),
        _ => {}
    }
    let mut out = [0.0; 4];
    let mut count = 0;
    // Skip a constructor's name, whose digit (`vec2`) is no value.
    let values = text.find('(').map_or(text, |open| &text[open..]);
    for piece in values.split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))) {
        if let (Ok(number), Some(slot)) = (piece.parse::<f32>(), out.get_mut(count)) {
            *slot = number;
            count += 1;
        }
    }
    (count > 0).then_some(out)
}

/// The numbers of `uniform` for a layer: what the layer's `uniforms` give (`given`), else its `@default`, else
/// zeros. `variable` looks up a `$name`.
#[must_use]
pub fn numbers(uniform: &Uniform, given: Option<&Value>, variable: &dyn Fn(&str) -> Option<Value>) -> [f32; 4] {
    let from = |value: &Value| -> Option<[f32; 4]> {
        match value {
            Value::Number(n) => Some([pen_doc::value::narrow(n.as_f64()?), 0.0, 0.0, 0.0]),
            Value::Bool(on) => Some([f32::from(u8::from(*on)), 0.0, 0.0, 0.0]),
            Value::String(text) => numbers_in(text),
            Value::Array(list) => {
                let mut out = [0.0; 4];
                for (slot, item) in out.iter_mut().zip(list) {
                    *slot = pen_doc::value::narrow(item.as_f64()?);
                }
                Some(out)
            }
            _ => None,
        }
    };
    let given = match given {
        Some(Value::String(text)) if text.starts_with('$') => variable(text.trim_start_matches('$')).as_ref().and_then(from),
        Some(value) => from(value),
        None => None,
    };
    let mut out = given.or_else(|| uniform.default.as_deref().and_then(numbers_in)).unwrap_or([0.0; 4]);
    // A colour without an alpha is opaque.
    if uniform.ty == Type::Vec(4) && matches!(given_text(given_or_default(uniform, None)), Some(text) if text.len() == 7) {
        out[3] = 1.0;
    }
    out
}

fn given_or_default<'a>(uniform: &'a Uniform, given: Option<&'a Value>) -> Option<&'a str> {
    given.and_then(Value::as_str).or(uniform.default.as_deref())
}

fn given_text(text: Option<&str>) -> Option<&str> {
    text.map(str::trim).filter(|t| t.starts_with('#'))
}

/// Where the sampler `uniform` gets its picture: the layer's value for it, else its `@default`.
#[must_use]
pub fn source(uniform: &Uniform, given: Option<&Value>) -> Source {
    if uniform.role == Role::Sdf {
        return Source::Nothing;
    }
    match given_or_default(uniform, given).map(str::trim) {
        Some("pencil:object") => Source::Object,
        Some("pencil:background") => Source::Background,
        Some(url) if !url.is_empty() => Source::File(url.to_owned()),
        _ => Source::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHECKER: &str = r"#version 100
precision mediump float;
/** @resolution */
uniform vec2 u_resolution;

/**
 * @label Size
 * @default 32
 * @range 1, 64
 */
uniform float u_size;

/**
 * @label Primary Color
 * @color
 * @default #ffffff
 */
uniform vec3 u_color1;
uniform bool u_flip;

void main() {
  vec2 cell = floor(gl_FragCoord.xy / u_size);
  float check = mod(cell.x + cell.y, 2.0);
  if (u_flip) { check = 1.0 - check; }
  gl_FragColor = vec4(u_color1 * check, 1.0);
}
";

    const PIXELATE: &str = r"/** @resolution */
uniform vec2 u_resolution;
/** @label Content */
uniform sampler2D u_content;
/** @default 8 */
uniform float u_pixelSize;
void main() {
  vec2 cell = (floor(gl_FragCoord.xy / u_pixelSize) + 0.5) * u_pixelSize;
  for (int i = 0; i < 2; i++) {
    gl_FragColor += 0.5 * texture2D( u_content , cell / u_resolution);
  }
  gl_FragColor.a *= float(textureSize(u_content, 0).x > 0);
}
";

    #[test]
    fn uniforms_are_read_with_their_notes() {
        let program = compile(CHECKER).unwrap();
        let names: Vec<&str> = program.uniforms.iter().map(|u| u.name.as_str()).collect();
        assert_eq!(names, ["u_resolution", "u_size", "u_color1", "u_flip"]);
        let size = &program.uniforms[1];
        assert_eq!(
            (size.ty, size.role, size.title(), size.default.as_deref(), size.range),
            (Type::Float, Role::Plain, "Size", Some("32"), (Some(1.0), Some(64.0)))
        );
        assert_eq!(program.uniforms[0].role, Role::Resolution);
        assert!(program.uniforms[2].color && !program.uniforms[3].color && program.uniforms[3].label.is_none());
    }

    #[test]
    fn a_webgl_shader_compiles_and_its_block_holds_every_uniform() {
        let program = compile(CHECKER).unwrap();
        // frame, then a vec2, a float beside it, a vec3 and the bool as an int in the vec3's fourth slot.
        assert_eq!(program.block_size, 48);
        let bytes = program.block([1.0, 2.0, 3.0, 4.0], &[], |u| match u.name.as_str() {
            "u_size" => [32.0, 0.0, 0.0, 0.0],
            "u_color1" => [0.25, 0.5, 0.75, 9.0],
            "u_flip" => [1.0, 0.0, 0.0, 0.0],
            _ => [640.0, 480.0, 0.0, 0.0],
        });
        let float = |at: usize| f32::from_ne_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
        assert_eq!([float(0), float(4), float(8), float(12)], [1.0, 2.0, 3.0, 4.0]);
        assert_eq!([float(16), float(20), float(24)], [640.0, 480.0, 32.0]);
        assert_eq!([float(32), float(36), float(40)], [0.25, 0.5, 0.75]);
        assert_eq!(i32::from_ne_bytes([bytes[44], bytes[45], bytes[46], bytes[47]]), 1);
    }

    #[test]
    fn samplers_are_split_and_their_calls_redirected() {
        let program = compile(PIXELATE).unwrap();
        assert!(program.glsl.contains("uniform texture2D pc_tex_u_content;") && program.glsl.contains("pc_sample_u_content( cell / u_resolution)"));
        assert!(program.glsl.contains("pc_size_u_content( 0)"));
        assert!(program.offsets.contains_key("pc_map_u_content"));
        assert_eq!(source(&program.uniforms[1], Some(&Value::String("pencil:object".to_owned()))), Source::Object);
        assert_eq!(source(&program.uniforms[1], None), Source::Nothing);
        assert_eq!(source(&program.uniforms[1], Some(&Value::String("./a.png".to_owned()))), Source::File("./a.png".to_owned()));
    }

    #[test]
    fn a_broken_shader_says_so_instead_of_reaching_the_gpu() {
        assert!(compile("void main() { gl_FragColor = vec4(nothing, 1.0); }").is_err());
        assert!(compile("uniform mat4 u_m; void main() {}").unwrap_err().contains("mat4"));
        assert!(compile("uniform float u_a[4]; void main() {}").is_err());
    }

    #[test]
    fn values_come_from_the_layer_then_the_default_then_zero() {
        let program = compile(CHECKER).unwrap();
        let none = |_: &str| None;
        let (size, color) = (&program.uniforms[1], &program.uniforms[2]);
        assert_eq!(numbers(size, None, &none), [32.0, 0.0, 0.0, 0.0]);
        assert_eq!(numbers(size, Some(&serde_json::json!(12)), &none), [12.0, 0.0, 0.0, 0.0]);
        assert_eq!(numbers(color, None, &none), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(numbers(color, Some(&serde_json::json!([0.5, 0.25, 1])), &none), [0.5, 0.25, 1.0, 0.0]);
        assert_eq!(numbers(color, Some(&serde_json::json!("#ff0000")), &none), [1.0, 0.0, 0.0, 1.0]);
        let variable = |name: &str| (name == "gap").then(|| serde_json::json!(7));
        assert_eq!(numbers(size, Some(&serde_json::json!("$gap")), &variable), [7.0, 0.0, 0.0, 0.0]);
        assert_eq!(numbers(&program.uniforms[3], None, &none), [0.0; 4]);
        assert_eq!(numbers_in("vec2(1.5, -2)"), Some([1.5, -2.0, 0.0, 0.0]));
        assert_eq!(numbers_in("true"), Some([1.0, 0.0, 0.0, 0.0]));
        assert_eq!(numbers_in("none"), None);
    }
}
