#!/usr/bin/env python3
"""One-off codemod for the Bevy 0.14 -> 0.19 UI bundle removals (M-0044).

Rewrites NodeBundle/ButtonBundle/ImageBundle/TextBundle::from_section into component tuples and
renames a few types. Brace-aware; run on files, review the diff, then fix the rest by compiler.
"""
import re
import sys


def match(s, i, open_c, close_c):
    """Index of the bracket closing the one at s[i]."""
    depth = 0
    in_str = False
    j = i
    while j < len(s):
        c = s[j]
        if in_str:
            if c == "\\":
                j += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == open_c:
            depth += 1
        elif c == close_c:
            depth -= 1
            if depth == 0:
                return j
        j += 1
    raise ValueError("unbalanced")


def split_top(body):
    """Split on top-level commas."""
    parts, depth, cur, in_str = [], 0, [], False
    i = 0
    while i < len(body):
        c = body[i]
        if in_str:
            cur.append(c)
            if c == "\\":
                i += 1
                cur.append(body[i])
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
            cur.append(c)
        elif c in "([{":
            depth += 1
            cur.append(c)
        elif c in ")]}":
            depth -= 1
            cur.append(c)
        elif c == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
        i += 1
    if "".join(cur).strip():
        parts.append("".join(cur))
    return [p.strip() for p in parts if p.strip()]


def fields(body):
    out = {}
    order = []
    for p in split_top(body):
        if p.startswith(".."):
            continue
        m = re.match(r"(\w+)\s*:\s*(.*)$", p, re.S)
        if m:
            out[m.group(1)] = m.group(2).strip()
        elif re.fullmatch(r"\w+", p):
            out[p] = p
        order.append(p)
    return out


def border(expr):
    m = re.fullmatch(r"BorderColor\((.*)\)", expr, re.S)
    return f"BorderColor::all({m.group(1)})" if m else expr


def conv_struct(s, name, builder):
    pat = re.compile(r"(?<!-> )(?<!\w)" + name + r"\s*\{")
    pos = 0
    while True:
        m = pat.search(s, pos)
        if not m:
            return s
        start = m.start()
        b = m.end() - 1
        e = match(s, b, "{", "}")
        body = s[b + 1 : e]
        new = builder(fields(body))
        s = s[:start] + new + s[e + 1 :]
        pos = start + len(new)


def extra(f, skip):
    comps = []
    for k, v in f.items():
        if k in skip:
            continue
        if k == "border_color":
            comps.append(border(v))
        elif k in ("background_color", "visibility", "transform", "focus_policy", "z_index", "global_transform"):
            comps.append(v)
        else:
            comps.append(f"/* TODO {k}: {v} */")
    return comps


def node_expr(f):
    return f.get("style", "Node::default()")


def build_node(f):
    return "(" + ", ".join([node_expr(f)] + extra(f, {"style"})) + ")"


def build_button(f):
    return "(" + ", ".join(["Button", node_expr(f)] + extra(f, {"style"})) + ")"


def build_image(f):
    img = f.get("image", "ImageNode::default()")
    return "(" + ", ".join([img, node_expr(f)] + extra(f, {"style", "image"})) + ")"


def build_pbr(f):
    comps = []
    if "mesh" in f:
        comps.append(f"Mesh3d({f['mesh']})")
    if "material" in f:
        comps.append(f"MeshMaterial3d({f['material']})")
    for k in ("transform", "global_transform", "visibility"):
        if k in f:
            comps.append(f[k])
    other = [k for k in f if k not in ("mesh", "material", "transform", "global_transform", "visibility")]
    comps += [f"/* TODO {k}: {f[k]} */" for k in other]
    return "(" + ", ".join(comps) + ")"


def build_spatial(f):
    comps = [f[k] for k in ("transform", "visibility") if k in f]
    return "(" + ", ".join(comps) + ")" if len(comps) != 1 else comps[0]


def build_light(key):
    def b(f):
        comps = [f.get(key, "Default::default()")]
        comps += [f[k] for k in ("transform", "visibility") if k in f]
        return "(" + ", ".join(comps) + ")"
    return b


def build_cam3d(f):
    comps = ["Camera3d::default()"]
    comps += [f[k] for k in ("camera", "transform", "projection", "tonemapping", "exposure") if k in f]
    return "(" + ", ".join(comps) + ")"


def conv_calls(s):
    s = re.sub(r"\bCamera2dBundle::default\(\)", "Camera2d", s)
    s = re.sub(r"\bDirectionalLightBundle::default\(\)", "DirectionalLight::default()", s)
    s = re.sub(r"\bSpatialBundle::default\(\)", "Transform::default()", s)
    pat = re.compile(r"\bSpatialBundle::from_transform\s*\(")
    pos = 0
    while True:
        m = pat.search(s, pos)
        if not m:
            break
        b = m.end() - 1
        e = match(s, b, "(", ")")
        inner = s[b + 1 : e]
        s = s[: m.start()] + inner + s[e + 1 :]
        pos = m.start() + len(inner)
    return s


def conv_text(s):
    pat = re.compile(r"\bTextBundle::from_section\s*\(")
    pos = 0
    while True:
        m = pat.search(s, pos)
        if not m:
            return s
        b = m.end() - 1
        e = match(s, b, "(", ")")
        args = split_top(s[b + 1 : e])
        text = args[0]
        style = args[1] if len(args) > 1 else "TextStyle::default()"
        sm = re.fullmatch(r"TextStyle\s*\{(.*)\}", style, re.S)
        comps = [f"Text::new({text})"]
        if sm:
            f = fields(sm.group(1))
            font = {k: v for k, v in f.items() if k in ("font", "font_size")}
            if font:
                inner = ", ".join(
                    f"{k}: FontSize::Px({v})" if k == "font_size" else f"{k}: {v}" for k, v in font.items()
                )
                comps.append(f"TextFont {{ {inner}, ..default() }}")
            else:
                comps.append("TextFont::default()")
            if "color" in f:
                comps.append(f"TextColor({f['color']})")
        elif style != "TextStyle::default()":
            comps.append(f"/* TODO text style {style} */")
        end = e + 1
        # chained builders
        rest = s[end:]
        cm = re.match(r"\s*\.with_style\(\s*Style\s*\{", rest)
        if cm:
            b2 = end + cm.end() - 1
            e2 = match(s, b2, "{", "}")
            comps.append("Node {" + s[b2 + 1 : e2] + "}")
            end = s.index(")", e2) + 1
        new = "(" + ", ".join(comps) + ")"
        s = s[:m.start()] + new + s[end:]
        pos = m.start() + len(new)


RENAMES = [
    (r"bevy::render::texture::", "bevy::image::"),
    (r"bevy::render::render_asset::RenderAssetUsages", "bevy::asset::RenderAssetUsages"),
    (r"bevy::render::render_resource::ShaderRef", "bevy::shader::ShaderRef"),
    (r"bevy::render::view::NoFrustumCulling", "bevy::camera::visibility::NoFrustumCulling"),
    (r"bevy::core::Name", "bevy::ecs::name::Name"),
    (r"bevy::hierarchy::", "bevy::ecs::hierarchy::"),
    (r"bevy::ecs::system::Resource", "bevy::ecs::resource::Resource"),
    (r"\bTransformSystem\b", "TransformSystems"),
    (r"\bInputSystem\b", "InputSystems"),
    (r"\bBuildChildren\b", "BuildChildren_REMOVED"),
    (r"\bEventWriter<", "MessageWriter<"),
    (r"\bEventReader<", "MessageReader<"),
    (r"\bEvents<", "Messages<"),
    (r"\.add_event::<", ".add_message::<"),
    (r"\b(?!tx\b|reply\b|jobs\b)(\w+)\.send\(", r"\1.write("),
    (r"\.send_default\(\)", ".write_default()"),
    (r"\.send_batch\(", ".write_batch("),
    (r"\.send_event\(", ".write_message("),
    (r"#\[derive\(([^)]*)\bEvent\b", r"#[derive(\1Message"),
    (r"\bBreakLineOn\b", "LineBreak"),
    (r"\.sections\[0\]\.value", ".0"),
    (r"\bUiImage\b", "ImageNode"),
    (r"\bChildBuilder\b", "ChildSpawnerCommands"),
    (r"\.despawn_recursive\(\)", ".despawn()"),
    (r"\.delta_seconds\(\)", ".delta_secs()"),
    (r"\.elapsed_seconds\(\)", ".elapsed_secs()"),
    (r"\.get_single_mut\(\)", ".single_mut()"),
    (r"\.get_single\(\)", ".single()"),
    (r"-> NodeBundle\b", "-> impl Bundle"),
    (r"\bNodeBundle\b(?=[,)>])", "Node"),
    (r"\bStyle\b(?=\s*[{,;)>]|\s*$)", "Node"),
]


def main():
    for path in sys.argv[1:]:
        s = open(path).read()
        o = s
        s = conv_struct(s, "NodeBundle", build_node)
        s = conv_struct(s, "ButtonBundle", build_button)
        s = conv_struct(s, "ImageBundle", build_image)
        s = conv_struct(s, "PbrBundle", build_pbr)
        s = conv_struct(s, "MaterialMeshBundle", build_pbr)
        s = conv_struct(s, "SpatialBundle", build_spatial)
        s = conv_struct(s, "PointLightBundle", build_light("point_light"))
        s = conv_struct(s, "SpotLightBundle", build_light("spot_light"))
        s = conv_struct(s, "DirectionalLightBundle", build_light("directional_light"))
        s = conv_struct(s, "Camera3dBundle", build_cam3d)
        s = conv_calls(s)
        s = conv_text(s)
        for a, b in RENAMES:
            s = re.sub(a, b, s)
        s = re.sub(r"-> Style\b", "-> Node", s)
        s = re.sub(r"\bStyle::", "Node::", s)
        if s != o:
            open(path, "w").write(s)
            print("rewrote", path)


main()
