"""Recording stand-in for `bpy`: every attribute is a MagicMock, so the
generated script's control flow, indexing, and data plumbing run for real
while Blender API calls are merely recorded."""
from unittest.mock import MagicMock

import mathutils

_root = MagicMock(name="bpy")
data = _root.data
ops = _root.ops
context = _root.context
app = _root.app
app.version_string = "stub-0.0"
types = _root.types

# `scene.view_layers[0]` must be a distinct object from `scene.view_layers.new(...)`
context.scene.view_layers.__getitem__ = MagicMock(return_value=MagicMock(name="main_layer"))
# `bpy.context.active_object` must return a fresh object per primitive call so
# per-object attributes (scale, parent, ...) don't collide.
_objects = []


def _fresh_object(*_args, **_kwargs):
    obj = MagicMock(name=f"object{len(_objects)}")
    obj.users_collection = []
    obj.scale = mathutils.Vector((1.0, 1.0, 1.0))
    _objects.append(obj)
    return None


for op in ("primitive_uv_sphere_add", "primitive_cube_add", "primitive_cylinder_add",
           "primitive_cone_add", "primitive_plane_add"):
    getattr(ops.mesh, op).side_effect = _fresh_object

type(context).active_object = property(lambda self: _objects[-1] if _objects else MagicMock())


def call_counts():
    return {
        "primitives": len(_objects),
        "cameras_new": data.cameras.new.call_count,
        "lights_new": data.lights.new.call_count,
        "materials_new": data.materials.new.call_count,
        "markers_new": context.scene.timeline_markers.new.call_count,
        "view_layers_new": context.scene.view_layers.new.call_count,
        "render_calls": ops.render.render.call_count,
        "compositor_groups": data.node_groups.new.call_count,
        "compositor_nodes": data.node_groups.new.return_value.nodes.new.call_count,
        "file_output_items": (
            data.node_groups.new.return_value.nodes.new.return_value
            .file_output_items.new.call_count
        ),
        "objects_new": data.objects.new.call_count,
    }
