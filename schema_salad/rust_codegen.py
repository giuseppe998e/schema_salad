"""Rust code generator for schema salad definitions."""

__all__ = ["RustCodeGen"]

import dataclasses
import functools
import itertools
import json
import re
import shutil
import sys
from abc import ABC, abstractmethod
from collections.abc import Iterator, MutableMapping, MutableSequence, Sequence
from importlib.resources import files as resource_files
from io import StringIO
from pathlib import Path
from time import sleep
from typing import (
    Any,
    ClassVar,
    Optional,
    TextIO,
    Union,
)

from . import _logger
from .avro.schema import (
    ArraySchema,
    EnumSchema,
    JsonDataType,
    NamedSchema,
    NamedUnionSchema,
    PrimitiveSchema,
    RecordSchema,
    Schema,
    UnionSchema,
    Field as SaladField,
    Names as SaladNames,
    make_avsc_object,
)
from .codegen_base import CodeGenBase
from .schema import make_valid_avro
from .validate import avro_shortname


def dataclass(*args, **kwargs):
    """
    A wrapper around `@dataclass` attribute that automatically enables
    `slots` if Python version >= 3.10.
    """
    if sys.version_info >= (3, 10):
        return dataclasses.dataclass(*args, slots=True, **kwargs)
    return dataclasses.dataclass(*args, **kwargs)


#
# Util Functions
#

__RUST_RESERVED_WORDS = [
    "type", "self", "let", "fn", "struct", "impl", "trait", "enum", "pub",
    "mut", "true", "false", "return", "match", "if", "else", "for", "in",
    "where", "ref", "use", "mod", "const", "static", "as", "move", "async",
    "await", "dyn", "loop", "break", "continue", "super", "crate", "unsafe",
    "extern", "box", "virtual", "override", "macro", "while", "yield",
    "typeof", "sizeof", "final", "pure", "abstract", "become", "do",
    "alignof", "offsetof", "priv", "proc", "unsized",
]  # fmt: skip

# __FIELD_NAME_REX_DICT = [
#     (re.compile(r"(?<=[a-z0-9])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])"), "_"),
#     (re.compile(r"([\W_]$)|\W"), lambda m: "" if m.group(1) else "_"),
#     (re.compile(r"^([0-9])"), lambda m: f"_{m.group(1)}"),
# ]
__TYPE_NAME_REX_DICT = [
    (re.compile(r"(?:^|[^a-zA-Z0-9.])(\w)"), lambda m: m.group(1).upper()),
    (re.compile(r"\.([a-zA-Z])"), lambda m: m.group(1).upper()),
    (re.compile(r"(?:^|\.)([0-9])"), lambda m: f"_{m.group(1)}"),
]
__MD_NON_HYPERLINK_REX = re.compile(
    r"(?<![\[(<\"])(\b[a-zA-Z]+://[a-zA-Z0-9\-.]+\.[a-zA-Z]{2,}(?::[0-9]+)?(?:/\S*)?)(?!\S*[])>\"])"
)


# TODO Check strings for Unicode standard for `XID_Start` and `XID_Continue`
# @functools.cache
def rust_sanitize_field_ident(value: str) -> str:
    """
    Checks whether the field name is a Rust reserved world, or escapes it.
    """
    # value = functools.reduce(lambda s, r: re.sub(*r, s), __FIELD_NAME_REX_DICT, value)
    # value = value.lower()
    if value in __RUST_RESERVED_WORDS:
        return f"r#{value}"
    return value


# TODO Check strings for Unicode standard for `XID_Start` and `XID_Continue`
@functools.cache
def rust_sanitize_type_ident(value: str) -> str:
    """
    Converts an input string into a valid Rust type name (PascalCase).
    Results are cached for performance optimization.
    """
    return functools.reduce(lambda s, r: re.sub(*r, s), __TYPE_NAME_REX_DICT, value)


def rust_sanitize_doc_iter(value: Union[list[str], str]) -> Iterator[str]:
    """
    Sanitizes Markdown doc-strings by splitting lines and wrapping non-hyperlinked
    URLs in angle brackets.
    """
    return map(
        lambda v: re.sub(__MD_NON_HYPERLINK_REX, lambda m: f"<{m.group()}>", v),
        itertools.chain.from_iterable(map(  # flat_map
            lambda v: v.rstrip().split("\n"),
            [value] if isinstance(value, str) else value,
        )),
    )  # fmt: skip


@functools.cache
def to_rust_literal(value: Any) -> str:
    """
    Convert Python values to their equivalent Rust literal representation.
    Results are cached for performance optimization.
    """
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, (int, float)):
        return str(value)
    if isinstance(value, str):
        if value.startswith('"') and value.endswith('"'):
            value = value[1:-1]
        return json.dumps(value, ensure_ascii=False)
    if isinstance(value, (list, tuple)):
        list_entries = ", ".join(map(to_rust_literal, value))
        return f"[{list_entries}]"
    if value is None:
        return "Option::None"
    raise TypeError(f"Unsupported type for Rust literal conversion: {type(value).__name__}")


def make_avro(items: MutableSequence[JsonDataType]) -> MutableSequence[NamedSchema]:
    """
    Processes a list of dictionaries to generate a list of Avro schemas.
    """

    # Same as `from .utils import convert_to_dict`, which, however, is not public
    def convert_to_dict(j4: Any) -> Any:
        """Convert generic Mapping objects to dicts recursively."""
        if isinstance(j4, MutableMapping):
            return {k: convert_to_dict(v) for k, v in j4.items()}
        if isinstance(j4, MutableSequence):
            return list(map(convert_to_dict, j4))
        return j4

    name_dict = {entry["name"]: entry for entry in items}
    avro = make_valid_avro(items, name_dict, set())
    avro = [
        t
        for t in avro
        if isinstance(t, MutableMapping)
        and not t.get("abstract")
        and t.get("type") != "org.w3id.cwl.salad.documentation"
    ]

    names = SaladNames()
    make_avsc_object(convert_to_dict(avro), names)
    return list(names.names.values())


#
# Rust AST Nodes
#


# ASSERT: The string is a valid Rust identifier.
RustIdent = str  # alias


@dataclass  # ASSERT: Immutable class
class RustLifetime:
    """
    Represents a Rust lifetime parameter (e.g., `'a`).
    """

    ident: RustIdent

    def __hash__(self) -> int:
        return hash(self.ident)

    def __str__(self) -> str:
        return f"'{str(self.ident)}"


class RustType(ABC):
    """
    Abstract class for Rust types.
    """

    pass


class RustMeta(ABC):
    """
    Abstract class for Rust attribute metas.
    """

    pass


@dataclass(unsafe_hash=True)  # ASSERT: Immutable class
class RustAttribute:
    """
    Represents a Rust attribute (e.g., `#[derive(Debug)]`).
    """

    meta: RustMeta

    def __str__(self) -> str:
        return f"#[{str(self.meta)}]"


RustAttributes = Sequence[RustAttribute]  # alias
RustAttributesMut = MutableSequence[RustAttribute]  # alias


RustGenerics = Sequence[Union[RustLifetime, RustType]]  # alias
RustGenericsMut = MutableSequence[Union[RustLifetime, RustType]]  # alias


@dataclass(unsafe_hash=True)  # ASSERT: Immutable class
class RustPathSegment:
    """
    Represents a segment in a Rust path with optional generics.
    """

    ident: RustIdent
    generics: RustGenerics = dataclasses.field(default_factory=tuple)

    REX: ClassVar[re.Pattern] = re.compile(r"^([a-zA-Z_]\w*)(?:<([ \w\t,'<>]+)>)?$")

    def __str__(self) -> str:
        if not self.generics:
            return self.ident
        generics = sorted(self.generics, key=lambda r: 0 if isinstance(r, RustLifetime) else 1)
        generics_str = ", ".join(map(str, generics))
        return f"{self.ident}<{generics_str}>"

    # noinspection PyArgumentList
    @classmethod
    @functools.cache
    def from_str(cls, value: str) -> "RustPathSegment":
        """
        Parses a string into RustPathSegment class.
        Results are cached for performance optimization.
        """

        def parse_generics_string(value_generics: str) -> RustGenerics:
            generics_sequence: Union[MutableSequence[str], RustGenerics] = []
            current, deep = [], 0
            for idx, char in enumerate(value_generics):
                deep += (char == "<") - (char == ">")
                if deep == 0 and char == ",":
                    generics_sequence.append("".join(current).strip())
                    current = []
                elif deep < 0:
                    raise ValueError(f"Poorly formatted Rust path generics: '{value}'.")
                else:
                    current.append(char)
            if deep > 0:
                raise ValueError(f"Poorly formatted Rust path generics: '{value}'.")
            generics_sequence.append("".join(current).strip())
            return tuple([
                RustLifetime(g[1:]) if g[0] == "'" else RustPath.from_str(g)
                for g in generics_sequence
            ])  # fmt: skip

        #
        # `from_str(...)` method
        if match := re.match(RustPathSegment.REX, value):
            ident, generics = match.groups()
            return cls(ident, parse_generics_string(generics) if generics else tuple())
        raise ValueError(f"Poorly formatted Rust path segment: '{value}'.")


RustPathSegments = Sequence[RustPathSegment]  # alias
RustPathSegmentsMut = MutableSequence[RustPathSegment]  # alias


@dataclass(unsafe_hash=True)  # ASSERT: Immutable class
class RustPath(RustType, RustMeta):
    """
    Represents a complete Rust path (e.g., `::std::vec::Vec<T>`).
    """

    # ASSERT: Never initialized with an empty sequence
    segments: RustPathSegments
    leading_colon: bool = False

    def __truediv__(self, other: Union["RustPath", RustPathSegment]) -> "RustPath":
        if isinstance(other, RustPath):
            if self.segments[-1].generics:
                raise ValueError("Cannot chain to a RustPath with generics.")
            if other.leading_colon:
                raise ValueError("Cannot chain a RustPath with leading colon.")
            return RustPath(
                segments=tuple([*self.segments, *other.segments]),
                leading_colon=self.leading_colon,
            )
        if isinstance(other, RustPathSegment):
            if self.segments[-1].generics:
                raise ValueError("Cannot chain to a RustPath with generics.")
            return RustPath(
                segments=tuple([*self.segments, other]),
                leading_colon=self.leading_colon,
            )
        raise TypeError(f"RustPath chaining with type `{type(other).__name__}` not supported.")

    def __str__(self) -> str:
        leading_colon = "::" if self.leading_colon else ""
        path_str = "::".join(map(str, self.segments))
        return leading_colon + path_str

    # noinspection PyArgumentList
    @classmethod
    @functools.cache
    def from_str(cls, value: str) -> "RustPath":
        """
        Parses a string into RustPath class.
        Results are cached for performance optimization.
        """
        norm_value, leading_colon = (value[2:], True) if value.startswith("::") else (value, False)
        segments, segment_with_generics = [], 0
        for segment in map(RustPathSegment.from_str, norm_value.split("::")):
            if len(segment.generics):
                segment_with_generics += 1
            segments.append(segment)
        if segment_with_generics > 1:
            raise ValueError(f"Poorly formatted Rust path: '{value}'")
        return cls(tuple(segments), leading_colon)

    # def parent(self) -> "RustPath":
    #     """
    #     Returns a new RustPath containing all but the last segment.
    #     """
    #     return RustPath(
    #         segments=self.segments[:-1],
    #         leading_colon=self.leading_colon,
    #     )


@dataclass(unsafe_hash=True)  # ASSERT: Immutable class
class RustTypeTuple(RustType):
    """
    Represents a Rust tuple type (e.g., `(T, U)`).
    """

    # ASSERT: Never initialized with an empty sequence
    types: Sequence[RustType]

    def __str__(self) -> str:
        types_str = ", ".join(str(ty) for ty in self.types)
        return f"({types_str})"


@dataclass  # ASSERT: Immutable class
class RustMetaList(RustMeta):
    """
    Represents attribute meta list information (e.g., `derive(Debug, Clone)`)
    """

    path: RustPath
    metas: Sequence[RustMeta] = tuple()

    def __hash__(self) -> int:
        return hash(self.path)

    def __str__(self) -> str:
        meta_str = ", ".join(str(meta) for meta in self.metas)
        return f"{str(self.path)}(" + meta_str + ")"


@dataclass  # ASSERT: Immutable class
class RustMetaNameValue(RustMeta):
    """
    Represents attribute meta name-value information (e.g., `key = value`)
    """

    path: RustPath
    value: Any = True

    def __hash__(self) -> int:
        return hash(self.path)

    def __str__(self) -> str:
        return f"{str(self.path)} = {to_rust_literal(self.value)}"


#
# Rust Type Representations
#


@dataclass
class RustNamedType(ABC):  # ABC class
    """
    Abstract class for Rust struct and enum types.
    """

    ident: RustIdent
    attrs: RustAttributes = dataclasses.field(default_factory=list)
    visibility: str = "pub"

    def __hash__(self) -> int:
        return hash(self.ident)

    @abstractmethod
    def write_to(self, writer: TextIO, depth: int = 0) -> None:
        pass

    def __str__(self) -> str:
        output = StringIO()
        self.write_to(output, 0)
        return output.getvalue()


@dataclass  # ASSERT: Immutable class
class RustField:
    """
    Represents a field in a Rust struct.
    """

    ident: RustIdent
    type: RustType
    attrs: RustAttributes = dataclasses.field(default_factory=list)

    def __hash__(self) -> int:
        return hash(self.ident)

    def write_to(self, writer: TextIO, depth: int = 0) -> None:
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join(f"{indent}{str(attr)}" for attr in self.attrs) + "\n")
        writer.write(f"{indent}{self.ident}: {str(self.type)}")


RustFields = Union[Sequence[RustField], RustTypeTuple]  # alias
RustFieldsMut = Union[MutableSequence[RustField], RustTypeTuple]  # alias


@dataclass
class RustStruct(RustNamedType):
    """
    Represents a Rust struct definition.
    """

    fields: Optional[RustFields] = None

    def write_to(self, writer: TextIO, depth: int = 0) -> None:
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join(f"{indent}{str(attr)}" for attr in self.attrs) + "\n")

        writer.write(f"{indent}{self.visibility} struct {self.ident}")
        if self.fields is None:
            writer.write(";\n")
        elif isinstance(self.fields, RustTypeTuple):
            writer.write(f"{str(self.fields)};\n")
        else:
            writer.write(" {\n")
            for field_ in self.fields:
                field_.write_to(writer, depth + 1)
                writer.write(",\n")
            writer.write(f"{indent}}}\n")


@dataclass  # ASSERT: Immutable class
class RustVariant:
    """
    Represents a variant in a Rust enum.
    """

    ident: RustIdent
    tuple: Optional[RustTypeTuple] = None
    attrs: RustAttributes = dataclasses.field(default_factory=list)

    def __hash__(self) -> int:
        return hash(self.ident)

    def write_to(self, writer: TextIO, depth: int = 0) -> None:
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join(f"{indent}{str(attr)}" for attr in self.attrs) + "\n")

        writer.write(f"{indent}{self.ident}")
        if self.tuple:
            writer.write(str(self.tuple))

    # noinspection PyArgumentList
    @classmethod
    def from_path(cls, path: RustPath) -> "RustVariant":
        ident = "".join(
            map(
                lambda p: p.segments[-1].ident,
                itertools.chain(
                    filter(lambda g: isinstance(g, RustPath), path.segments[-1].generics),
                    itertools.repeat(path, 1),
                ),
            )
        )
        ident = ident.replace("StrValue", "String", 1)  # HACK
        return cls(ident=ident, tuple=RustTypeTuple([path]))


RustVariants = Sequence[RustVariant]  # alias
RustVariantsMut = MutableSequence[RustVariant]  # alias


@dataclass
class RustEnum(RustNamedType):
    """
    Represents a Rust enum definition.
    """

    variants: RustVariants = dataclasses.field(default_factory=tuple)

    def write_to(self, writer: TextIO, depth: int = 0) -> None:
        indent = "    " * depth

        if self.attrs:
            writer.write("\n".join(f"{indent}{str(attr)}" for attr in self.attrs) + "\n")

        writer.write(f"{indent}{self.visibility} enum {self.ident} {{\n")
        for variant in self.variants:
            variant.write_to(writer, depth + 1)
            writer.write(",\n")
        writer.write(f"{indent}}}\n")


# Wrapper for the RustNamedType `write_to()` method call
def salad_macro_write_to(ty: RustNamedType, writer: TextIO, depth: int = 0) -> None:
    """
    Writes a RustNamedType wrapping it in the Schema Salad macro
    """
    indent = "    " * depth
    writer.write(indent + "salad_core::define_type! {\n")
    ty.write_to(writer, 1)
    writer.write(indent + "}\n\n")


#
# Rust Module Tree
#


@dataclass
class RustModuleTree:
    """
    Represents a Rust module with submodules and named types
    """

    ident: RustIdent  # ASSERT: Immutable field
    parent: "RustModuleTree"  # ASSERT: Immutable field
    named_types: MutableMapping[RustIdent, RustNamedType] = dataclasses.field(default_factory=dict)
    submodules: MutableMapping[RustIdent, "RustModuleTree"] = dataclasses.field(
        default_factory=dict
    )

    def __hash__(self) -> int:
        return hash((self.ident, self.parent))

    def get_rust_path(self) -> RustPath:
        """
        Returns the complete Rust path from root to this module.
        """
        segments, current = [], self
        while current:
            segments.append(RustPathSegment(ident=current.ident))
            current = current.parent
        return RustPath(segments=tuple(reversed(segments)))

    def add_submodule(self, path: Union[RustPath, str]) -> "RustModuleTree":
        """
        Creates a new submodule or returns an existing one with the given path.
        """
        if isinstance(path, str):
            path = RustPath.from_str(path)
        segments = iter(path.segments)

        # First segment, corner case
        if (first := next(segments, None)) is None:
            return self

        if first.ident == self.ident:
            current = self
        else:
            current = self.submodules.setdefault(
                first.ident,
                RustModuleTree(ident=first.ident, parent=self),
            )

        # Subsequent segments
        for segment in segments:
            current = current.submodules.setdefault(
                segment.ident,
                RustModuleTree(ident=segment.ident, parent=current),
            )
        return current

    # def get_submodule(self, path: Union[RustPath, str]) -> Optional["RustModuleTree"]:
    #     """
    #     Returns a submodule from this module tree by its Rust path, if any.
    #     """
    #     if isinstance(path, str):
    #         path = RustPath.from_str(path)
    #     current, last_segment_idx = self, len(path.segments) - 1
    #     for idx, segment in enumerate(path.segments):
    #         if (idx == last_segment_idx) and (current.ident == segment.ident):
    #             return current
    #         current = current.submodules.get(segment.ident)
    #         if not current:
    #             return None
    #     return None

    def add_named_type(self, ty: RustNamedType) -> RustPath:
        """
        Adds a named type to this module tree and returns its complete Rust path.
        Raises `ValueError` if type with same name already exists
        """
        module_rust_path = self.get_rust_path()
        if ty.ident in self.named_types:
            raise ValueError(f"Duplicate Rust type '{ty.ident}' in '{module_rust_path}'.")
        self.named_types[ty.ident] = ty
        return module_rust_path / RustPathSegment(ident=ty.ident)

    # def get_named_type(self, path: RustPath) -> Optional[RustNamedType]:
    #     if module := self.get_submodule(path.parent()):
    #         return module.named_types.get(path.segments[-1].ident)
    #     return None

    def write_to_fs(self, base_path: Path) -> None:
        """
        Writes the module tree to the filesystem under the given base path.
        """

        # noinspection PyShadowingNames
        def write_module_file(module: "RustModuleTree", path: Path, mode: str = "wt") -> None:
            with open(path, mode=mode) as module_rs:
                if module.submodules:
                    module_rs.write(
                        "\n".join([f"mod {mod.ident};" for mod in module.submodules.values()])
                        + "\n\n"
                    )
                if module.named_types:
                    for ty in module.named_types.values():
                        salad_macro_write_to(ty, module_rs, 0)

        #
        # `write_to_fs(...)` method
        path = base_path.resolve()
        traversing_stack: MutableSequence[tuple[Path, RustModuleTree]] = []

        # Write `lib.rs` module (corner case)
        if not self.parent:
            path.mkdir(mode=0o755, parents=True, exist_ok=True)
            write_module_file(module=self, path=path / "lib.rs", mode="at")
            traversing_stack.extend((path, sub_mod) for sub_mod in self.submodules.values())
        else:
            traversing_stack.append((path, self))

        # Generate module files
        while traversing_stack:
            path_parent, module = traversing_stack.pop()

            if not module.submodules:
                path_parent.mkdir(mode=0o755, parents=True, exist_ok=True)
                write_module_file(module=module, path=path_parent / f"{module.ident}.rs")
                continue

            path_module = path_parent / module.ident
            path_module.mkdir(mode=0o755, parents=True, exist_ok=True)
            write_module_file(module=module, path=path_module / "mod.rs")
            traversing_stack.extend(
                (path_module, sub_mod) for sub_mod in module.submodules.values()
            )


#
# Salad Core Types
#


def rust_type_option(rust_ty: RustPath) -> RustPath:
    # noinspection PyArgumentList
    return RustPath([RustPathSegment(ident="Option", generics=[rust_ty])])


def rust_type_list(rust_ty: RustPath) -> RustPath:
    # noinspection PyArgumentList
    return RustPath([
        RustPathSegment(ident="crate"),
        RustPathSegment(ident="core"),
        RustPathSegment(ident="List", generics=[rust_ty]),
    ])  # fmt: skip


_AVRO_TO_RUST_PRESET = {
    # Salad Types
    "boolean": RustPath.from_str("crate::core::Bool"),
    "int": RustPath.from_str("crate::core::Int"),
    "long": RustPath.from_str("crate::core::Long"),
    "float": RustPath.from_str("crate::core::Float"),
    "double": RustPath.from_str("crate::core::Double"),
    "string": RustPath.from_str("crate::core::StrValue"),
    "org.w3id.cwl.salad.Any": RustPath.from_str("crate::core::Any"),
    "org.w3id.cwl.salad.ArraySchema.type.Array_name": RustPath.from_str("crate::TypeArray"),
    "org.w3id.cwl.salad.EnumSchema.type.Enum_name": RustPath.from_str("crate::TypeEnum"),
    "org.w3id.cwl.salad.RecordSchema.type.Record_name": RustPath.from_str("crate::TypeRecord"),
    # CWL Types
    "org.w3id.cwl.cwl.Expression": RustPath.from_str("crate::core::StrValue"),
}


#
# Code generator
#


class RustCodeGen(CodeGenBase):
    """
    Rust code generator for schema salad definitions.
    """

    # Static
    PACKAGE_VERSION = "0.1.0"  # Version of the generated crate
    __TEMPLATE_DIR = Path(str(resource_files("schema_salad").joinpath("rust"))).resolve()

    # Parsing related
    __avro_to_rust: MutableMapping[str, RustPath]
    __document_root_paths: MutableSequence[RustPath]
    __module_tree: RustModuleTree
    __schema_stack: MutableSequence[NamedSchema]

    # noinspection PyMissingConstructor
    def __init__(
        self,
        base_uri: str,
        package: str,
        salad_version: str,
        target: Optional[str] = None,
    ) -> None:
        self.package = package
        self.PACKAGE_VERSION = self.__generate_crate_version(salad_version)
        self.output_dir = Path(target or ".").resolve()
        self.document_root_attr = RustAttribute(
            meta=RustMetaList(
                path=RustPath.from_str("salad"),
                metas=[
                    RustPath.from_str("root"),
                    RustMetaNameValue(
                        path=RustPath.from_str("base_uri"),
                        value=base_uri,
                    ),
                ],
            )
        )

    def parse(self, items: MutableSequence[JsonDataType]) -> None:
        # Create output directory
        self.__init_output_directory()

        # Generate Rust named types
        self.__avro_to_rust = _AVRO_TO_RUST_PRESET.copy()
        self.__document_root_paths = []
        self.__module_tree = RustModuleTree(ident="crate", parent=None)
        self.__schema_stack = list(reversed(make_avro(items)))

        while self.__schema_stack:
            schema = self.__schema_stack.pop()

            if not schema.name.startswith(self.package):
                continue
            if schema.name in self.__avro_to_rust:
                _logger.warn(f"Skip parse step for schema: {schema.name}")
                continue

            rust_path = self.__parse_named_schema(schema)
            self.__avro_to_rust[schema.name] = rust_path

        # Generate `DocumentRoot` enum
        self.__module_tree.add_named_type(
            RustEnum(
                ident="DocumentRoot",
                attrs=[self.document_root_attr],
                variants=list(map(RustVariant.from_path, self.__document_root_paths)),
            )
        )

        # Write named types to the "src" folder
        self.__module_tree.write_to_fs(self.output_dir / "src")

    def __parse_named_schema(self, named: NamedSchema) -> RustPath:
        if isinstance(named, RecordSchema):
            return self.__parse_record_schema(named)
        if isinstance(named, EnumSchema):
            return self.__parse_enum_schema(named)
        if isinstance(named, NamedUnionSchema):
            return self.__parse_union_schema(named)
        raise ValueError(f"Cannot parse schema of type {type(named).__name__}.")

    def __parse_record_schema(self, record: RecordSchema) -> RustPath:
        ident = rust_sanitize_type_ident(avro_shortname(record.name))
        attrs, _ = self.__parse_named_schema_attrs(record)
        fields = set(self.__parse_record_field(f, record) for f in record.fields)

        if record.get_prop("documentRoot"):
            attrs = [*attrs, self.document_root_attr]

        rust_path = self.__module_tree \
            .add_submodule(self.__get_submodule_path(record)) \
            .add_named_type(RustStruct(ident=ident, attrs=attrs, fields=fields))  # fmt: skip

        if record.get_prop("documentRoot"):
            self.__document_root_paths.append(rust_path)
        return rust_path

    def __parse_record_field(self, field: SaladField, parent: RecordSchema) -> RustField:
        def parse_field_type(schema: Schema) -> RustPath:
            if isinstance(schema, UnionSchema):
                filtered_schemas = [s for s in schema.schemas if s.type != "null"]
                filtered_schemas_len = len(filtered_schemas)

                if filtered_schemas_len == 1:
                    rust_path = parse_field_type(filtered_schemas[0])
                    if filtered_schemas_len < len(schema.schemas):
                        return rust_type_option(rust_path)
                    return rust_path

                union_name = f"{parent.name}.{field.name}"
                if rust_path := self.__avro_to_rust.get(union_name):
                    if filtered_schemas_len < len(schema.schemas):
                        return rust_type_option(rust_path)
                    return rust_path

                named_union_schema = NamedUnionSchema.__new__(NamedUnionSchema)
                setattr(named_union_schema, "_props", getattr(schema, "_props"))
                setattr(named_union_schema, "_schemas", filtered_schemas)
                named_union_schema.set_prop("name", union_name)
                named_union_schema.set_prop("namespace", parent.name)
                named_union_schema.set_prop("doc", field.get_prop("doc"))

                self.__schema_stack.append(named_union_schema)
                type_path = self.__get_submodule_path(named_union_schema) / RustPathSegment(
                    rust_sanitize_type_ident(avro_shortname(union_name))
                )
                if filtered_schemas_len < len(schema.schemas):
                    return rust_type_option(type_path)
                return type_path

            if isinstance(schema, (RecordSchema, EnumSchema)):
                return self.__avro_to_rust.get(
                    schema.name,
                    self.__get_submodule_path(schema)
                    / RustPathSegment(ident=rust_sanitize_type_ident(avro_shortname(schema.name))),
                )

            if isinstance(schema, ArraySchema):
                return rust_type_list(parse_field_type(schema.items))

            if isinstance(schema, PrimitiveSchema):
                return self.__avro_to_rust.get(schema.type)

            raise ValueError(f"Cannot parse schema with type: '{type(schema).__name__}'.")

        #
        # `__parse_record_field(...)` method
        ident = rust_sanitize_field_ident(field.name)
        attrs, _ = self.__parse_field_schema_attrs(field)
        ty = parse_field_type(field.type)
        return RustField(ident=ident, attrs=attrs, type=ty)

    def __parse_union_schema(self, union: NamedUnionSchema) -> RustPath:
        def parse_variant_array_subtype(schema: Schema) -> RustPath:
            if isinstance(schema, UnionSchema):
                filtered_schemas = [s for s in schema.schemas if s.type != "null"]

                item_name = f"{union.name}_item"
                named_union_schema = NamedUnionSchema.__new__(NamedUnionSchema)
                setattr(named_union_schema, "_props", getattr(schema, "_props"))
                setattr(named_union_schema, "_schemas", filtered_schemas)
                named_union_schema.set_prop("name", item_name)
                named_union_schema.set_prop("namespace", union.name)

                self.__schema_stack.append(named_union_schema)
                return self.__get_submodule_path(named_union_schema) / RustPathSegment(
                    rust_sanitize_type_ident(avro_shortname(item_name))
                )

            if isinstance(schema, (RecordSchema, EnumSchema)):
                return self.__avro_to_rust.get(
                    schema.name,
                    self.__get_submodule_path(schema)
                    / RustPathSegment(ident=rust_sanitize_type_ident(avro_shortname(schema.name))),
                )

            if isinstance(schema, PrimitiveSchema):
                return self.__avro_to_rust.get(schema.type)

        def parse_variant_type(schema: Schema) -> RustVariant:
            if isinstance(schema, (RecordSchema, EnumSchema)):
                return RustVariant.from_path(
                    self.__avro_to_rust.get(
                        schema.name,
                        self.__get_submodule_path(schema)
                        / RustPathSegment(
                            ident=rust_sanitize_type_ident(avro_shortname(schema.name))
                        ),
                    )
                )

            if isinstance(schema, PrimitiveSchema):
                return RustVariant.from_path(self.__avro_to_rust.get(schema.type))

            if isinstance(schema, ArraySchema):
                return RustVariant.from_path(
                    rust_type_list(parse_variant_array_subtype(schema.items))
                )

            raise ValueError(f"Cannot parse schema with type: '{type(schema).__name__}'.")

        #
        # `__parse_union_schema(...)` method
        ident = rust_sanitize_type_ident(avro_shortname(union.name))
        attrs, _ = self.__parse_named_schema_attrs(union)
        variants = set(map(parse_variant_type, union.schemas))

        return self.__module_tree \
            .add_submodule(self.__get_submodule_path(union)) \
            .add_named_type(RustEnum(ident=ident, attrs=attrs, variants=variants))  # fmt: skip

    def __parse_enum_schema(self, enum: EnumSchema) -> RustPath:
        ident = rust_sanitize_type_ident(avro_shortname(enum.name))
        attrs, docs_count = self.__parse_named_schema_attrs(enum)
        attrs = [
            *attrs,
            RustAttribute(
                RustMetaList(
                    path=RustPath.from_str("derive"),
                    metas=[RustPath.from_str("Copy")],
                )
            ),
        ]

        if len(enum.symbols) == 1:
            return self.__module_tree \
                .add_submodule(self.__get_submodule_path(enum)) \
                .add_named_type(
                    RustStruct(
                        ident=ident,
                        attrs=[
                            *attrs[:docs_count],
                            RustAttribute(
                                RustMetaNameValue(
                                    path=RustPath.from_str("doc"),
                                    value=f"Matches constant value `{enum.symbols[0]}`.",
                                )
                            ),
                            *attrs[docs_count:],
                            RustAttribute(
                                RustMetaList(
                                    path=RustPath.from_str("salad"),
                                    metas=[RustMetaNameValue(
                                        path=RustPath.from_str("as_str"),
                                        value=enum.symbols[0],
                                    )],
                                )
                            ),
                        ],
                    )
                )  # fmt: skip
        else:
            return self.__module_tree \
                .add_submodule(self.__get_submodule_path(enum)) \
                .add_named_type(
                    RustEnum(
                        ident=ident,
                        attrs=attrs,
                        variants=[
                            RustVariant(
                                ident=rust_sanitize_type_ident(symbol),
                                attrs=[
                                    RustAttribute(
                                        RustMetaNameValue(
                                            path=RustPath.from_str("doc"),
                                            value=f"Matches constant value `{symbol}`.",
                                        )
                                    ),
                                    RustAttribute(
                                        RustMetaList(
                                            path=RustPath.from_str("salad"),
                                            metas=[RustMetaNameValue(
                                                path=RustPath.from_str("as_str"),
                                                value=symbol,
                                            )],
                                        )
                                    ),
                                ],
                            )
                            for symbol in enum.symbols
                        ],
                    )
                )  # fmt: skip

    # End of named schemas parse block
    #
    @staticmethod
    def __parse_named_schema_attrs(schema: NamedSchema) -> tuple[RustAttributes, int]:
        attrs, docs_count = [], 0

        if docs := schema.get_prop("doc"):
            rust_path_doc = RustPath.from_str("doc")
            attrs.extend(
                RustAttribute(RustMetaNameValue(path=rust_path_doc, value=doc))
                for doc in rust_sanitize_doc_iter(docs)
            )
            docs_count = len(attrs)

        attrs.append(
            RustAttribute(
                RustMetaList(
                    path=RustPath.from_str("derive"),
                    metas=[
                        RustPath.from_str("Debug"),
                        RustPath.from_str("Clone"),
                    ],
                )
            )
        )

        return attrs, docs_count

    @staticmethod
    def __parse_field_schema_attrs(schema: SaladField) -> tuple[RustAttributes, int]:
        attrs, docs_count = [], 0

        if docs := schema.get_prop("doc"):
            rust_path_doc = RustPath.from_str("doc")
            attrs.extend(
                RustAttribute(RustMetaNameValue(path=rust_path_doc, value=doc))
                for doc in rust_sanitize_doc_iter(docs)
            )
            docs_count = len(attrs)

        metas = []
        if default := schema.get_prop("default"):
            metas.append(RustMetaNameValue(path=RustPath.from_str("default"), value=default))
        if jsonld_predicate := schema.get_prop("jsonldPredicate"):
            if isinstance(jsonld_predicate, str) and jsonld_predicate == "@id":
                metas.append(RustPath.from_str("identifier"))
            elif isinstance(jsonld_predicate, MutableMapping):
                metas.extend(
                    RustMetaNameValue(path=RustPath.from_str(rust_path), value=value)
                    for key, rust_path in [
                        ("mapSubject", "map_key"),
                        ("mapPredicate", "map_predicate"),
                        ("subscope", "subscope"),
                    ]
                    if (value := jsonld_predicate.get(key))
                )
        if metas:
            attrs.append(RustAttribute(RustMetaList(path=RustPath.from_str("salad"), metas=metas)))

        return attrs, docs_count

    # End of attributes parse block
    #
    def __get_submodule_path(self, schema: NamedSchema) -> RustPath:
        segments = [RustPathSegment(ident="crate")]
        if namespace_prop := schema.get_prop("namespace"):
            if (namespace := namespace_prop.removeprefix(self.package)) not in ("", "."):
                namespace_segment = namespace.split(".")[1].lower()
                module_ident = rust_sanitize_field_ident(namespace_segment)
                segments.append(RustPathSegment(ident=module_ident))
        return RustPath(segments=segments)

    def __init_output_directory(self) -> None:
        """
        Initialize the output directory structure.
        """
        if self.output_dir.is_file():
            raise ValueError(f"Output directory cannot be a file: {self.output_dir}")
        if not self.output_dir.exists():
            _logger.info(f"Creating output directory: {self.output_dir}")
            self.output_dir.mkdir(mode=0o755, parents=True)
        elif any(self.output_dir.iterdir()):
            _logger.warning(
                f"Output directory is not empty: {self.output_dir}.\n"
                "Wait for 3 seconds before proceeding..."
            )
            sleep(3)

        def copy2_wrapper(src: str, dst: str) -> object:
            if not src.endswith("rust/Cargo.toml"):
                return shutil.copy2(src, dst)

            replace_dict = [
                ("{package_name}", self.output_dir.name),
                ("{package_version}", self.PACKAGE_VERSION),
            ]

            with open(src, "r") as src, open(dst, "w") as dst:
                content = src.read()
                for placeholder, value in replace_dict:
                    content = content.replace(placeholder, value)
                dst.write(content)

        shutil.copytree(
            RustCodeGen.__TEMPLATE_DIR,
            self.output_dir,
            dirs_exist_ok=True,
            copy_function=copy2_wrapper,
        )

    @staticmethod
    def __generate_crate_version(salad_version: str) -> str:
        salad_version = salad_version.removeprefix("v")
        return f"{RustCodeGen.PACKAGE_VERSION}+salad{salad_version}"
