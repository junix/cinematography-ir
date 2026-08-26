"""Minimal mathutils stand-in so generated bpy scripts can be exercised
without Blender. Only what template.py uses."""
import math


class Quaternion:
    def __init__(self, values=(1.0, 0.0, 0.0, 0.0)):
        self.values = tuple(float(v) for v in values)


class Vector:
    def __init__(self, values=(0.0, 0.0, 0.0)):
        values = tuple(float(v) for v in values)
        if len(values) != 3:
            raise ValueError(f"Vector expects 3 components, got {len(values)}")
        self.x, self.y, self.z = values

    def __iter__(self):
        return iter((self.x, self.y, self.z))

    def __len__(self):
        return 3

    def __getitem__(self, index):
        return (self.x, self.y, self.z)[index]

    def __add__(self, other):
        return Vector((self.x + other.x, self.y + other.y, self.z + other.z))

    def __sub__(self, other):
        return Vector((self.x - other.x, self.y - other.y, self.z - other.z))

    def __mul__(self, scalar):
        return Vector((self.x * scalar, self.y * scalar, self.z * scalar))

    __rmul__ = __mul__

    def __truediv__(self, scalar):
        return Vector((self.x / scalar, self.y / scalar, self.z / scalar))

    @property
    def length(self):
        return math.sqrt(self.x * self.x + self.y * self.y + self.z * self.z)

    def to_track_quat(self, track, up):
        assert track in "XYZ-" and up in "XYZ"
        return Quaternion()

    def __repr__(self):
        return f"Vector(({self.x}, {self.y}, {self.z}))"


class Matrix:
    def __init__(self, rows):
        self.rows = tuple(tuple(float(v) for v in row) for row in rows)
        if len(self.rows) != 3 or any(len(r) != 3 for r in self.rows):
            raise ValueError("Matrix expects 3 rows of 3")
        # Sanity: columns must be (nearly) unit vectors — catches bad slicing.
        for c in range(3):
            norm = math.sqrt(sum(self.rows[r][c] ** 2 for r in range(3)))
            if abs(norm - 1.0) > 1e-3:
                raise ValueError(f"column {c} of basis is not unit length: {norm}")

    def to_quaternion(self):
        return Quaternion()
