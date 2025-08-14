"""Python classes representing a Rust Abstract Syntax Tree (AST)."""

# pylint: disable=too-few-public-methods,too-many-positional-arguments,too-many-arguments

import functools
import json
from abc import ABC, abstractmethod
from collections import deque
from collections.abc import Sequence
from enum import Enum as PyEnum
from enum import auto
from io import StringIO
from typing import IO, Any, Optional, Union


class Node(ABC):
    """Base class for all AST nodes."""


class Ident(Node):
    """An identifier like `foo` or `bar`."""

    __slots__ = ("__value",)

    def __init__(self, value: str) -> None:
        """Initialize an identifier with a value.

        Args:
            value: The string value of the identifier
        """
        self.__value = value

    def __str__(self) -> str:
        """Return the string representation of the identifier."""
        return self.__value

    def __hash__(self) -> int:
        """Return the hash of the identifier based on its value."""
        return hash(self.__value)


class Lifetime(Node):
    """A Rust lifetime like `'a`."""

    __slots__ = ("__value",)

    def __init__(self, value: str) -> None:
        """Initialize a lifetime with a value.

        Args:
            value: The string value of the lifetime
        """
        value_sanitized = value.removeprefix("'")
        self.__value = f"'{value_sanitized}"

    def __str__(self) -> str:
        """Return the string representation of the lifetime."""
        return self.__value

    def __hash__(self) -> int:
        """Return the hash of the lifetime based on its value."""
        return hash(self.__value)


class Lit:
    """A Rust literal value like 42, "hello", or true."""

    __slots__ = ("__repr",)

    def __init__(self, value: Any) -> None:
        """Initialize a literal with a value.

        Args:
            value: The value to convert to a Rust literal
        """
        self.__repr = Lit.__into_literal(value)

    def __str__(self) -> str:
        """Return the string representation of the literal."""
        return self.__repr

    def __hash__(self) -> int:
        """Return the hash of the literal based on its string representation."""
        return hash(self.__repr)

    @staticmethod
    @functools.cache
    def __into_literal(value: Any) -> str:
        if isinstance(value, bool):
            return str(value).lower()
        if isinstance(value, (int, float)):
            return str(value)
        if isinstance(value, str):
            return json.dumps(value, ensure_ascii=False)
        if isinstance(value, (list, tuple)):
            list_entries = ", ".join(Lit.__into_literal(val) for val in value)
            return f"[{list_entries}]"
        if value is None:
            return "Option::None"
        raise TypeError(f"Unsupported type for Rust literal conversion: {type(value).__name__}")


class PathSegment(Node):
    """A segment of a path: `foo` in `foo::bar::baz`."""

    __slots__ = ("ident", "args")

    def __init__(
        self, ident: Ident, args: Optional[Sequence[Union[Lifetime, "Type"]]] = None
    ) -> None:
        """Initialize a path segment.

        Args:
            ident: The identifier of the segment
            args: Optional sequence of type arguments
        """
        super().__init__()
        self.ident = ident
        self.args = args or []

    def __str__(self) -> str:
        """Return the string representation of the path segment, with type arguments if present."""
        if self.args:
            args_sorted = sorted(
                self.args, reverse=True, key=lambda arg: int(isinstance(arg, Lifetime))
            )
            args_str = ", ".join(str(arg) for arg in args_sorted)
            return f"{self.ident}<{args_str}>"
        return str(self.ident)

    def __hash__(self) -> int:
        """Return the hash of the path segment based on identifier and type arguments."""
        return hash((self.ident, self.args))

    @classmethod
    @functools.lru_cache(maxsize=8)  # It's unlikely to be used on its own
    def from_str(cls, segment: str) -> "PathSegment":
        """Parse a string representation of a path segment."""
        raise RuntimeError("unimplemented")


class Path(Node):
    """A Rust path (`foo::bar::Baz`)."""

    __slots__ = ("segments", "leading_colon")

    def __init__(self, segments: Sequence[PathSegment], leading_colon: bool = False) -> None:
        """Initialize a path with segments.

        Args:
            segments: Sequence of path segments
            leading_colon: Whether the path starts with '::'
        """
        super().__init__()
        self.segments = segments
        self.leading_colon = leading_colon

    def __str__(self) -> str:
        """Return the string representation of the path."""
        result = "::" if self.leading_colon else ""
        return result + "::".join(str(seg) for seg in self.segments)

    def __hash__(self) -> int:
        """Return the hash of the path based on segments and leading colon flag."""
        return hash((tuple(self.segments), self.leading_colon))

    @classmethod
    @functools.cache
    def from_str(cls, path: str) -> "Path":
        """Parse a string representation of a Rust path into a Path object."""
        segments_str, leading_colon = (path[2:], True) if path.startswith("::") else (path, False)
        segment_strings = deque(segments_str.split("::"))

        segments = []
        while segment_strings:
            segment_str = segment_strings.popleft()
            segment = PathSegment.from_str(segment_str)

            if segment.args and segment_strings:
                raise ValueError(f"Poorly formatted Rust path: '{path}'")
            segments.append(segment)

        return Path(tuple(segments), leading_colon)


class Type(Node, ABC):  # pylint: disable=too-few-public-methods
    """Base class for Rust types."""


class TypePath(Type):
    """A type path like `std::vec::Vec<T>`."""

    __slots__ = ("__path",)

    def __init__(self, segments: Sequence[PathSegment], leading_colon: bool = False) -> None:
        """Initialize a TypePath with a Path object.

        Args:
            segments: Sequence of path segments
            leading_colon: Whether the path starts with '::'
        """
        self.__path = Path(segments, leading_colon)

    def __str__(self) -> str:
        """Return string representation of the type path."""
        return str(self.__path)

    def __hash__(self) -> int:
        """Return hash of this TypePath."""
        return hash(self.__path)

    def to_path(self) -> Path:
        """Return the underlying Path object."""
        return self.__path

    @property
    def segments(self) -> Sequence[PathSegment]:
        """Return the path segments."""
        return self.__path.segments

    @classmethod
    def from_str(cls, path: str) -> "TypePath":
        """Parse a string representation into a TypePath object."""
        path_i = Path.from_str(path)
        return cls(path_i.segments, path_i.leading_colon)


class TypeRef(Type):
    """A reference type like `&T` or `&mut T`."""

    __slots__ = ("ty", "lifetime", "is_mut")

    def __init__(
        self, ty: "Type", lifetime: Optional[Lifetime] = None, mutable: bool = False
    ) -> None:
        """Initialize a reference type.

        Args:
            ty: The referenced type
            lifetime: Optional lifetime annotation
            mutable: Whether the reference is mutable
        """
        super().__init__()
        self.ty = ty
        self.lifetime = lifetime
        self.mutable = mutable

    def __str__(self) -> str:
        """Return the string representation of the reference type."""
        mut_str = "mut " if self.mutable else ""
        lifetime_str = f"{self.lifetime} " if self.lifetime else ""
        return f"&{lifetime_str}{mut_str}{self.ty}"

    def __hash__(self) -> int:
        """Return the hash of the reference based on type, lifetime, and mutability."""
        return hash((self.ty, self.lifetime, self.mutable))


class TypeSlice(Type):
    """A slice type like `[T]`."""

    __slots__ = ("ty",)

    def __init__(self, ty: "Type") -> None:
        """Initialize a slice type.

        Args:
            ty: The type of elements in the slice
        """
        super().__init__()
        self.ty = ty

    def __str__(self) -> str:
        """Return the string representation of the slice type."""
        return f"[{self.ty}]"

    def __hash__(self) -> int:
        """Return the hash of the slice based on its element type."""
        return hash(self.ty)


class TypeArray(Type):
    """An array type like `[T; N]`."""

    __slots__ = ("ty", "len")

    def __init__(self, ty: "Type", length: int) -> None:
        """Initialize an array type.

        Args:
            ty: The type of elements in the array
            len: The length of the array
        """
        super().__init__()
        self.ty = ty
        self.len = length

    def __str__(self) -> str:
        """Return the string representation of the array type."""
        return f"[{self.ty}; {self.len}]"

    def __hash__(self) -> int:
        """Return the hash of the array based on element type and length."""
        return hash((self.ty, self.len))


class TypeTuple(Type):
    """A tuple type like `(T, U)`."""

    __slots__ = ("items",)

    def __init__(self, items: Sequence["Type"]) -> None:
        """Initialize a tuple type.

        Args:
            items: Sequence of types in the tuple
        """
        super().__init__()
        self.items = items

    def __str__(self) -> str:
        """Return the string representation of the tuple type."""
        elements_str = ", ".join(str(ty) for ty in self.items)
        return f"({elements_str})"

    def __hash__(self) -> int:
        """Return the hash of the tuple based on its item types."""
        return hash(tuple(self.items))


class Expr(Node, ABC):
    """Base class for Rust expressions."""


class ExprPath(Expr):
    """A path expression like `foo::bar`."""

    __slots__ = ("__path",)

    def __init__(self, segments: Sequence[PathSegment], leading_colon: bool = False) -> None:
        """Initialize an ExprPath with a Path object.

        Args:
            segments: Sequence of path segments
            leading_colon: Whether the path starts with '::'
        """
        self.__path = Path(segments, leading_colon)

    def __str__(self) -> str:
        """Return string representation of the expression path."""
        return str(self.__path)

    def __hash__(self) -> int:
        """Return hash of this ExprPath."""
        return hash(self.__path)

    def to_path(self) -> Path:
        """Return the underlying Path object."""
        return self.__path

    @property
    def segments(self) -> Sequence[PathSegment]:
        """Return the path segments."""
        return self.__path.segments

    @classmethod
    def from_str(cls, path: str) -> "ExprPath":
        """Parse a string representation into an ExprPath object."""
        path_i = Path.from_str(path)
        return cls(path_i.segments, path_i.leading_colon)


class ExprLit(Expr):
    """A literal expression like `42`."""

    __slots__ = ("__lit",)

    def __init__(self, lit: Any) -> None:
        """Initialize an ExprLit with a literal value.

        Args:
            lit: The literal value for this expression
        """
        self.__lit = lit if isinstance(lit, Lit) else Lit(lit)

    def __str__(self) -> str:
        """Return string representation of the literal expression."""
        return str(self.__lit)

    def __hash__(self) -> int:
        """Return hash of this ExprLit."""
        return hash(self.__lit)

    def to_lit(self) -> Lit:
        """Return the underlying Lit object."""
        return self.__lit


class AttrStyle(PyEnum):
    """Style of attributes."""

    OUTER = auto()  # `#[...]`
    INNER = auto()  # `#![...]`

    def __hash__(self) -> int:
        """Return the hash of the attribute style based on its value."""
        return hash(self.value)


class Attribute(Node):
    """A Rust attribute like `#[derive(Debug)]`."""

    __slots__ = ("meta", "style")

    def __init__(self, meta: "Meta", style: AttrStyle = AttrStyle.OUTER) -> None:
        """Initialize an attribute.

        Args:
            meta: The meta content of the attribute
            style: The style of the attribute (inner or outer)
        """
        super().__init__()
        self.meta = meta
        self.style = style

    def __str__(self) -> str:
        """Return the string representation of the attribute."""
        prefix = "![" if self.style == AttrStyle.INNER else "["
        return f"#{prefix}{self.meta}]"

    def __hash__(self) -> int:
        """Return the hash of the attribute based on style and meta content."""
        return hash((self.style, self.meta))

    @classmethod
    @functools.cache
    def from_str(cls, path: str) -> "Attribute":
        """Parse a string representation of a Rust attribute into a Attribute object."""
        raise RuntimeError("unimplemented")


class Meta(Node, ABC):  # pylint: disable=too-few-public-methods
    """A general meta item in attributes."""


class MetaPath(Meta):
    """A meta path item like `Clone`."""

    __slots__ = ("__path",)

    def __init__(self, segments: Sequence[PathSegment], leading_colon: bool = False) -> None:
        """Initialize a MetaPath with a Path object.

        Args:
            segments: Sequence of path segments
            leading_colon: Whether the path starts with '::'
        """
        self.__path = Path(segments, leading_colon)

    def __str__(self) -> str:
        """Return string representation of the meta path."""
        return str(self.__path)

    def __hash__(self) -> int:
        """Return hash of this MetaPath."""
        return hash(self.__path)

    def to_path(self) -> Path:
        """Return the underlying Path object."""
        return self.__path

    @property
    def segments(self) -> Sequence[PathSegment]:
        """Return the path segments."""
        return self.__path.segments

    @classmethod
    def from_str(cls, path: str) -> "MetaPath":
        """Parse a string representation into a MetaPath object."""
        path_i = Path.from_str(path)
        return cls(path_i.segments, path_i.leading_colon)


class MetaSequence(Meta):
    """A list in attributes like `derive(Debug, Clone)`."""

    __slots__ = ("path", "nested")

    def __init__(self, path: Path, nested: Sequence["Meta"]) -> None:
        """Initialize a meta sequence.

        Args:
            path: The path of the meta item
            nested: Sequence of nested meta items
        """
        super().__init__()
        self.path = path
        self.nested = nested

    def __str__(self) -> str:
        """Return the string representation of the meta sequence."""
        nested_str = ", ".join(str(meta) for meta in self.nested)
        return f"{str(self.path)}({nested_str})"

    def __hash__(self) -> int:
        """Return the hash of the meta sequence based on path and nested items."""
        return hash((self.path, tuple(self.nested)))


class MetaNameValue(Meta):
    """A name-value pair in attributes like `feature = "nightly"`."""

    __slots__ = ("path", "value")

    def __init__(self, path: Path, value: "Expr") -> None:
        """Initialize a name-value pair meta item.

        Args:
            path: The path of the meta item
            value: The expression value
        """
        super().__init__()
        self.path = path
        self.value = value

    def __str__(self) -> str:
        """Return the string representation of the name-value meta item."""
        return f"{self.path} = {str(self.value)}"

    def __hash__(self) -> int:
        """Return the hash of the name-value meta item based on path and value."""
        return hash((self.path, self.value))


class Generic(Node, ABC):
    """A generic parameter in a generic parameter list."""


class GenericLifetime(Generic):
    """A lifetime parameter like `'a` in `fn foo<'a>()`."""

    __slots__ = ("lifetime", "bounds")

    def __init__(
        self,
        lifetime: Lifetime,
        bounds: Optional[Sequence[Lifetime]] = None,
    ) -> None:
        """Initialize a lifetime generic parameter.

        Args:
            lifetime: The lifetime identifier
            bounds: Optional sequence of lifetime bounds
        """
        super().__init__()
        self.lifetime = lifetime
        self.bounds = bounds or []

    def __str__(self) -> str:
        """Return the string representation of the lifetime generic parameter."""
        if self.bounds:
            bounds_str = " + ".join(str(b) for b in self.bounds)
            return f"{self.lifetime}: {bounds_str}"
        return str(self.lifetime)

    def __hash__(self) -> int:
        """Return the hash of the lifetime generic based on lifetime and bounds."""
        return hash((self.lifetime, self.bounds))


class GenericType(Generic):
    """A type parameter like `T` in `fn foo<T>()`."""

    __slots__ = ("ident", "bounds")

    def __init__(
        self,
        ident: Ident,
        bounds: Optional[Sequence[Type]] = None,
    ) -> None:
        """Initialize a type generic parameter.

        Args:
            ident: The type identifier
            bounds: Optional sequence of trait bounds
        """
        super().__init__()
        self.ident = ident
        self.bounds = bounds or []

    def __str__(self) -> str:
        """Return the string representation of the type generic parameter."""
        if self.bounds:
            bounds_str = " + ".join(str(b) for b in self.bounds)
            return f"{self.ident}: {bounds_str}"
        return str(self.ident)

    def __hash__(self) -> int:
        """Return the hash of the type generic based on identifier and bounds."""
        return hash((self.ident, self.bounds))


class Visibility(PyEnum):
    """Rust visibility modifiers."""

    PUBLIC = "pub "
    CRATE = "pub(crate) "
    INHERITED = ""  # No explicit visibility

    def __str__(self) -> str:
        """Return the string representation of the visibility modifier."""
        return self.value


class Item(Node, ABC):
    """Base class for Rust items (struct, field, enum, variant)."""

    __slots__ = ("attrs", "vis")

    def __init__(
        self,
        attrs: Optional[Sequence[Attribute]],
        vis: Visibility,
    ) -> None:
        """Initialize a Rust item.

        Args:
            attrs: Optional sequence of attributes
            vis: Visibility modifier for the item
        """
        super().__init__()
        self.attrs = attrs or []
        self.vis = vis

    def __str__(self) -> str:
        """Return the string representation of the item."""
        output = StringIO()
        self.write_to(output, 0)
        return output.getvalue()

    def __hash__(self) -> int:
        """Return the hash of the item based on attributes and visibility."""
        return hash((self.attrs, self.vis))

    @abstractmethod
    def write_to(self, writer: IO[str], depth: int = 0) -> None:
        """Write field to the provided writer at the specified indent depth.

        Args:
            writer: The IO writer to write to
            depth: Indentation depth
        """


class Field(Item):
    """A struct field like `x: i32` or a field in a pattern."""

    __slots__ = ("ident", "ty")

    def __init__(
        self,
        ident: Ident,
        ty: Type,
        attrs: Optional[Sequence[Attribute]] = None,
        vis: Visibility = Visibility.INHERITED,
    ) -> None:
        """Initialize a struct field.

        Args:
            ident: The field identifier
            ty: The type of the field
            attrs: Optional sequence of attributes
            vis: Visibility modifier for the field
        """
        super().__init__(attrs, vis)
        self.ident = ident
        self.ty = ty

    def __hash__(self) -> int:
        """Return the hash of the field based on parent hash, identifier, and type."""
        return hash((super().__hash__(), self.ident, self.ty))

    def write_to(self, writer: IO[str], depth: int = 0) -> None:
        """Write field to the provided writer at the specified indent depth.

        Args:
            writer: The IO writer to write to
            depth: Indentation depth
        """
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join(f"{indent}{a}" for a in self.attrs) + "\n")

        writer.write(f"{indent}{self.vis}{self.ident}: {self.ty},\n")


class Struct(Item):
    """A struct item like `struct Foo { x: i32 }`."""

    __slots__ = ("ident", "fields", "generics")

    def __init__(
        self,
        ident: Ident,
        fields: Optional[Union[Sequence[Field], TypeTuple]] = None,
        generics: Optional[Sequence[Generic]] = None,
        attrs: Optional[Sequence[Attribute]] = None,
        vis: Visibility = Visibility.INHERITED,
    ) -> None:
        """Initialize a struct item.

        Args:
            ident: The struct identifier
            fields: Optional sequence of fields or tuple type
            generics: Optional sequence of generic parameters
            attrs: Optional sequence of attributes
            vis: Visibility modifier for the struct
        """
        super().__init__(attrs, vis)
        self.ident = ident
        self.fields = fields or []
        self.generics = generics or []

    def __hash__(self) -> int:
        """Return the hash of the struct based on parent hash, identifier, fields, and generics."""
        return hash((super().__hash__(), self.ident, self.fields, self.generics))

    def write_to(self, writer: IO[str], depth: int = 0) -> None:
        """Write struct to the provided writer at the specified indent depth.

        Args:
            writer: The IO writer to write to
            depth: Indentation depth
        """
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join([f"{indent}{str(attr)}" for attr in self.attrs]) + "\n")

        writer.write(f"{indent}{self.vis}struct {self.ident}")

        if self.generics:
            generics_sorted = sorted(
                self.generics, reverse=True, key=lambda gen: int(isinstance(gen, Lifetime))
            )
            generics_str = ", ".join(str(gen) for gen in generics_sorted)
            writer.write(f"<{generics_str}>")

        if isinstance(self.fields, TypeTuple):
            writer.write(f"{str(self.fields)};\n")
        elif len(self.fields) == 0:
            writer.write(";\n")
        else:
            writer.write(" {\n")
            for field in self.fields:
                field.write_to(writer, depth + 1)
            writer.write(f"{indent}}}")


class Variant(Item):
    """A variant in an enum like `Some(T)` in `enum Option<T> { Some(T), None }`."""

    __slots__ = ("ident", "fields", "discriminant")

    def __init__(
        self,
        ident: Ident,
        fields: Optional[Union[TypeTuple, Sequence[Field]]] = None,
        discriminant: Optional[Lit] = None,
        attrs: Optional[Sequence[Attribute]] = None,
    ) -> None:
        """Initialize an enum variant.

        Args:
            ident: The variant identifier
            fields: Optional sequence of fields or tuple type
            discriminant: Optional literal value for the variant
            attrs: Optional sequence of attributes
        """
        super().__init__(attrs, Visibility.INHERITED)
        self.ident = ident
        self.fields = fields or []
        self.discriminant = discriminant

    def __hash__(self) -> int:
        """Return the hash of the variant based on parent hash, ident, fields, and discriminant."""
        return hash((super().__hash__(), self.ident, self.fields, self.discriminant))

    def write_to(self, writer: IO[str], depth: int = 0) -> None:
        """Write variant to the provided writer at the specified indent depth.

        Args:
            writer: The IO writer to write to
            depth: Indentation depth
        """
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join(f"{indent}{a}" for a in self.attrs) + "\n")

        writer.write(f"{indent}{self.ident}")

        if isinstance(self.fields, TypeTuple):
            writer.write(f"{str(self.fields)},\n")
        elif len(self.fields) > 0:
            writer.write(" {\n")
            for field in self.fields:
                field.write_to(writer, depth + 1)
            writer.write(f"{indent}}},\n")
        else:
            if self.discriminant:
                writer.write(f" = {self.discriminant}")
            writer.write(",\n")


class Enum(Item):
    """An enum item like `enum Option<T> { Some(T), None }`."""

    __slots__ = ("ident", "variants", "generics")

    def __init__(
        self,
        ident: Ident,
        variants: Sequence[Variant],
        generics: Optional[Sequence[Generic]] = None,
        attrs: Optional[Sequence[Attribute]] = None,
        vis: Visibility = Visibility.INHERITED,
    ) -> None:
        """Initialize an enum item.

        Args:
            ident: The enum identifier
            variants: Sequence of enum variants
            generics: Optional sequence of generic parameters
            attrs: Optional sequence of attributes
            vis: Visibility modifier for the enum
        """
        super().__init__(attrs, vis)
        self.ident = ident
        self.variants = variants
        self.generics = generics or []

    def __hash__(self) -> int:
        """Return the hash of the enum based on parent hash, identifier, variants, and generics."""
        return hash((super().__hash__(), self.ident, self.variants, self.generics))

    def write_to(self, writer: IO[str], depth: int = 0) -> None:
        """Write enum to the provided writer at the specified indent depth.

        Args:
            writer: The IO writer to write to
            depth: Indentation depth
        """
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join([f"{indent}{str(attr)}" for attr in self.attrs]) + "\n")

        writer.write(f"{indent}{self.vis}enum {self.ident}")

        if self.generics:
            generics_sorted = sorted(
                self.generics, reverse=True, key=lambda gen: int(isinstance(gen, Lifetime))
            )
            generics_str = ", ".join(str(gen) for gen in generics_sorted)
            writer.write(f"<{generics_str}>")

        if self.variants:
            writer.write(" {\n")
            for variant in self.variants:
                variant.write_to(writer, depth + 1)
            writer.write(f"{indent}}}")
        else:
            writer.write(" { }")
