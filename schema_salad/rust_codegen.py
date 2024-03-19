"""Rust code generator for a given schema salad definition."""

import json
import os
import re
import shutil
from abc import ABC, abstractmethod
from io import StringIO, TextIOWrapper
from pathlib import Path
from typing import (
    Any,
    Dict,
    List,
    MutableMapping,
    Optional,
    TextIO,
    Type,
    Union,
)

from .avro.schema import (
    Schema,
    ArraySchema,
    EnumSchema,
    NamedSchema,
    PrimitiveSchema,
    RecordSchema,
    UnionSchema,
    Field as SaladField,
    Names as SaladNames,
    make_avsc_object,
)
from .codegen_base import CodeGenBase
from .exceptions import SchemaException
from .metaschema import parser_info
from .schema import make_valid_avro
from .utils import convert_to_dict, files
from .validate import avro_shortname

#
# Util functions
#

RUST_RESERVED_WORDS = [
    "as", "async", "await", "break", "const", "continue", "crate",
    "dyn", "else", "enum", "extern", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "Self", "self", "static", "struct",
    "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract",
    "alignof", "become", "box", "do", "final", "macro", "offsetof",
    "override", "priv", "proc", "pure", "sizeof", "typeof", "unsized", "virtual", "yield",
]
RUST_SAFE_VARIANT = [
    (re.compile("(^|[-_.])([a-zA-Z])"), lambda m: m.group(2).capitalize()),
    (re.compile("(^|[-.])([0-9])"), lambda m: f"_{m.group(2)}"),
    (re.compile("([a-zA-Z])[-_.]([0-9])"), lambda m: m.group(1) + m.group(2)),
]


def dict_get(d: Dict[str, Any], key: str, *types: Type) -> Any:
    """
    Retrieves a value from a dictionary by key and validates that it matches one
    of the specified types.
    """
    if not types:
        raise ValueError("At least one type must be specified.")
    val = d.get(key)
    if not any(isinstance(val, ty) for ty in types):
        expected_types = " | ".join(t.__name__ for t in types)
        raise TypeError(f"{key}: {type(val).__name__} != {expected_types}")
    return val


def make_avro(items: List[Dict[str, Any]]) -> List[NamedSchema]:
    """
    Processes a list of dictionaries to generate a list of Avro schemas.
    """
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
    return [v for v in names.names.values()]


def safe_file_copy(src: Path, dst: Path, replace: Optional[Dict[str, str]] = None) -> None:
    """
    Safely copies a file from the source path to the destination path,
    ensuring any necessary directories along the path exist. Optionally,
    performs string replacement in the copied file according to the provided
    dictionary.
    """
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(src, dst)

    if replace is not None:
        with open(dst, "r+") as f:
            content = f.read()

            for placeholder, value in replace.items():
                content = content.replace(placeholder, value)

            f.truncate(0)
            f.seek(0)
            f.write(content)


def safe_field_name(name: str) -> str:
    """
    Returns a modified version of the input name if it is reserved.
    """
    if name in RUST_RESERVED_WORDS:
        return f"r#{name}"
    return name


def safe_rust_name(value: str) -> str:
    """
    Takes a string as input and applies a series of replacements
    based on patterns defined in `RUST_SAFE_VARIANT`.
    """
    for pattern, repl in RUST_SAFE_VARIANT:
        value = re.sub(pattern, repl, value)
    return value


def generate_salad_attrs(attrs: Dict[str, Any]) -> List[str]:
    result = []
    keys = list(attrs.keys())

    for i in range(0, len(keys), 3):
        chunk_keys = keys[i : i + 3]
        chunk_str = ", ".join(
            f"{key}{f' = {json.dumps(attrs[key])}' if attrs[key] is not None else ''}"
            for key in chunk_keys
        )
        result.append(f"#[salad({chunk_str})]")

    return result


def generate_docs(docs: Union[str, List[str]]) -> List[str]:
    """
    Generates documentation comments for a given string or list of strings.
    """
    return [
        f"/// {sub_line}"
        for line in (docs.strip().splitlines(True) if isinstance(docs, str) else docs)
        for sub_line in line.splitlines()
    ]


#
# IOWrapper util class
#


class IOWrapper:
    def __init__(self, buf: Union[StringIO, TextIO, TextIOWrapper]) -> None:
        if not isinstance(buf, (StringIO, TextIO, TextIOWrapper)):
            raise IOError(f"Expected a writable text file object, got {type(buf).__name__}.")
        if not buf.writable():
            raise IOError(f"File '{buf.name}' is not writable.")
        self.buf = buf

    def __enter__(self) -> "IOWrapper":
        return self

    def __exit__(self, ty, value, traceback) -> None:
        self.close()

    @classmethod
    def open(cls, file, mode="w", **kwargs) -> "IOWrapper":
        return cls(open(file=file, mode=mode, **kwargs))

    def write(self, text: str, indent: int = 0) -> int:
        prefix = " " * indent
        return self.buf.write(prefix + text)

    def write_line(self, text: str, indent: int = 0) -> int:
        return self.write(f"{text}\n", indent)

    def close(self) -> None:
        self.buf.close()


class CodeProducer(ABC):
    @abstractmethod
    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        pass

    def to_string(self, depth: int = 0) -> str:
        buf = StringIO()
        with IOWrapper(buf) as writer:
            self.write_to(writer, depth)
            return buf.getvalue()


#
# Rust types
#


class RustType(CodeProducer):
    def __init__(
        self,
        name: str,
        namespace: Optional[str] = None,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
        variant: Optional[str] = None,
    ) -> None:
        self._name = name
        self._namespace = namespace
        self._docs = generate_docs(docs) if docs is not None else []
        self._attributes = attributes if attributes is not None else []
        self._variant = variant if variant is not None else name.split("::")[-1]

        if isinstance(salad_attrs, dict):
            self._attributes.extend(generate_salad_attrs(salad_attrs))

    def __str__(self) -> str:
        return self.name

    def __eq__(self, other) -> bool:
        return (
            isinstance(other, RustType)
            and self.name == other.name
            and self.namespace == other.namespace
        )

    def __hash__(self) -> int:
        return hash(self.name, self.namespace)

    @property
    def name(self) -> str:
        return self._name

    @property
    def namespace(self) -> Optional[str]:
        return self._namespace

    @property
    def docs(self) -> Optional[List[str]]:
        return self._docs

    @property
    def attributes(self) -> Optional[List[str]]:
        return self._attributes

    @property
    def variant(self) -> Optional[str]:
        return self._variant

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        pass


class StructRustType(RustType):
    def __init__(
        self,
        name: str,
        fields: List["StructField"],
        namespace: Optional[str] = None,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
        variant: Optional[str] = None,
    ) -> None:
        super().__init__(
            name=safe_rust_name(avro_shortname(name)),
            namespace=namespace,
            docs=docs,
            attributes=attributes,
            salad_attrs=salad_attrs,
            variant=variant,
        )
        self.fields = fields
        self._attributes.append("#[derive(Clone, Debug)]")

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        depth2 = 4 * (depth + 1)
        depth3 = depth2 * 2

        writer.write_line("\nschema_salad_macro::define_type! {", depth)

        for doc in self.docs:
            writer.write_line(doc, depth2)
        for attr in self.attributes:
            writer.write_line(attr, depth2)

        writer.write_line(f"pub struct {self.name} {{", depth2)
        for field in self.fields:
            field.write_to(writer, depth3)
        writer.write_line("}", depth2)

        writer.write_line("}", depth)


class StructField(CodeProducer):
    def __init__(
        self,
        name: str,
        ty: RustType,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
    ) -> None:
        self.name = self.safe_field_name(name)
        self.ty = ty
        self.docs = generate_docs(docs) if docs is not None else []
        self.attributes = attributes if attributes is not None else []

        if isinstance(salad_attrs, dict):
            self.attributes.extend(generate_salad_attrs(salad_attrs))

    @staticmethod
    def safe_field_name(name: str) -> str:
        if name in RUST_RESERVED_WORDS:
            return f"r#{name}"
        return name

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        for doc in self.docs:
            writer.write_line(doc, depth)
        for attr in self.attributes:
            writer.write_line(attr, depth)

        writer.write_line(f"{self.name}: {str(self.ty)},", depth)


class UnionRustType(RustType):
    def __init__(
        self,
        name: str,
        value: str,
        namespace: Optional[str] = None,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
        variant: Optional[str] = None,
    ) -> None:
        super().__init__(
            name=safe_rust_name(avro_shortname(name)),
            namespace=namespace,
            docs=docs,
            attributes=attributes,
            salad_attrs=salad_attrs,
            variant=variant,
        )
        self.value = value

        if docs is not None:
            self._docs.append("/// ")
        self._docs.append(f"/// Matches constant value `{self.value}`.")

        self._attributes.append(f"#[salad(as_str = \"{self.value}\")]")
        self._attributes.append("#[derive(Clone, Copy, Debug)]")

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        depth2 = 4 * (depth + 1)
        writer.write_line("\nschema_salad_macro::define_type! {", depth)

        for doc in self.docs:
            writer.write_line(doc, depth2)
        for attr in self.attributes:
            writer.write_line(attr, depth2)

        writer.write_line(f"pub struct {self.name};", depth2)
        writer.write_line("}", depth)


class EnumRustType(RustType):
    def __init__(
        self,
        name: str,
        variants: List["EnumVariant"],
        namespace: Optional[str] = None,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
        variant: Optional[str] = None,
    ) -> None:
        super().__init__(
            name=safe_rust_name(avro_shortname(name)),
            namespace=namespace,
            docs=docs,
            attributes=attributes,
            salad_attrs=salad_attrs,
            variant=variant,
        )
        self.variants = variants
        self._attributes.append("#[derive(Clone, Debug)]")

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        depth2 = 4 * (depth + 1)
        depth3 = depth2 * 2

        writer.write_line("\nschema_salad_macro::define_type! {", depth)

        for doc in self.docs:
            writer.write_line(doc, depth2)
        for attr in self.attributes:
            writer.write_line(attr, depth2)

        writer.write_line(f"pub enum {self.name} {{", depth2)
        for variant in self.variants:
            variant.write_to(writer, depth3)
        writer.write_line("}", depth2)

        writer.write_line("}", depth)


class EnumVariant(CodeProducer):
    def __init__(
        self,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
    ) -> None:
        self.docs = generate_docs(docs) if docs is not None else []
        self.attributes = attributes if attributes is not None else []

        if isinstance(salad_attrs, dict):
            self.attributes.extend(generate_salad_attrs(salad_attrs))


class EnumUnitVariant(EnumVariant):
    def __init__(
        self,
        value: str,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
    ) -> None:
        super().__init__(docs, attributes, salad_attrs)
        self.name = safe_rust_name(value)
        self.value = value

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        writer.write_line(f"/// Matches constant value `{self.value}`.", depth)
        writer.write_line(f'#[salad(as_str = "{self.value}")]', depth)
        writer.write_line(f"{self.name},", depth)


class EnumTupleVariant(EnumVariant):
    def __init__(
        self,
        ty: RustType,
        docs: Optional[Union[str, List[str]]] = None,
        attributes: Optional[List[str]] = None,
        salad_attrs: Optional[Dict[str, Any]] = None,
    ) -> None:
        super().__init__(docs, attributes, salad_attrs)
        self.ty = ty

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        for attr in self.attributes:
            writer.write_line(attr, depth)

        # TODO Check if 'OptionRustType' can be a variant
        writer.write_line(f"{self.ty.variant}({str(self.ty)}),", depth)


class RustTypeRef(RustType):
    def __init__(
        self,
        key: str,
        names: "RustNames",
    ) -> None:
        super().__init__("type_ref")
        self.names = names
        self.key = key

    def __str__(self) -> str:
        ty: RustType = self.names.get(self.key)
        return str(ty)

    @property
    def name(self) -> str:
        ty: RustType = self.names.get(self.key)
        return ty.name

    @property
    def namespace(self) -> Optional[str]:
        ty: RustType = self.names.get(self.key)
        return ty.namespace

    @property
    def docs(self) -> Optional[List[str]]:
        ty: RustType = self.names.get(self.key)
        return ty.docs

    @property
    def attributes(self) -> Optional[List[str]]:
        ty: RustType = self.names.get(self.key)
        return ty.attributes

    @property
    def variant(self) -> Optional[str]:
        ty: RustType = self.names.get(self.key)
        return ty.variant

    def get(self) -> Optional[RustType]:
        return self.names.get(self.key, True)

    def write_to(self, writer: IOWrapper, depth: int = 0) -> None:
        ty: RustType = self.names.get(self.key)
        ty.write_to(writer, depth)


class ArrayRustType(RustType):
    def __init__(
        self,
        sub_ty: RustType,
        variant: Optional[str] = None,
    ) -> None:
        super().__init__("array")
        self._variant = variant  # overrides "RustType" value
        self.sub_ty = sub_ty

    def __str__(self) -> str:
        return f"std::boxed::Box<[{self.name}]>"

    @property
    def name(self) -> str:
        return self.sub_ty.name

    @property
    def namespace(self) -> Optional[str]:
        return self.sub_ty.namespace

    @property
    def docs(self) -> Optional[List[str]]:
        return self.sub_ty.doc

    @property
    def variant(self) -> Optional[str]:
        if self._variant:
            return self._variant
        return f"{self.sub_ty.variant}List"


class OptionRustType(RustType):
    def __init__(self, sub_ty: RustType) -> None:
        super().__init__("option")
        self.sub_ty = sub_ty

    def __str__(self) -> str:
        return f"std::option::Option<{str(self.sub_ty)}>"

    @property
    def name(self) -> str:
        return self.sub_ty.name

    @property
    def namespace(self) -> Optional[str]:
        return self.sub_ty.namespace

    @property
    def docs(self) -> Optional[List[str]]:
        return self.sub_ty.doc

    @property
    def variant(self) -> Optional[str]:
        return self.sub_ty.variant


#
# Rust types dictionary
#


class RustNames:
    def __init__(self, init: Optional[Dict[str, RustType]] = None) -> None:
        self.items: Dict[str, RustType] = (
            {k: v for k, v in init.items()} if init is not None else {}
        )

    def __iter__(self):
        return iter(self.items.values())

    def has(self, key: str) -> bool:
        return bool(key in self.items)

    def add(self, key: str, ty: RustType) -> RustType:
        if not self.has(key):
            self.items[key] = ty
        return RustTypeRef(key, self)

    def get(self, key: str, opt: bool = False) -> Optional[RustType]:
        if not opt and not self.has(key):
            raise Exception(f"Rust type with key '{key}' not found.")
        return self.items.get(key)

    def get_ref(self, key: str) -> RustType:
        return RustTypeRef(key, self)


#
# Rust primitive types
#

BOOL_RUST_TYPE = RustType(
    name="crate::core::Bool",
    docs="A binary value.",
)

INT_RUST_TYPE = RustType(
    name="crate::core::Int",
    docs="32-bit signed integer.",
)

LONG_RUST_TYPE = RustType(
    name="crate::core::Long",
    docs="64-bit signed integer.",
)

FLOAT_RUST_TYPE = RustType(
    name="crate::core::Float",
    docs="Single precision (32-bit) IEEE 754 floating-point number.",
)

DOUBLE_RUST_TYPE = RustType(
    name="crate::core::Double",
    docs="Double precision (64-bit) IEEE 754 floating-point number.",
)

STRING_RUST_TYPE = RustType(
    name="crate::core::StrValue",
    docs="Unicode character sequence.",
    variant="String",
)

ANY_RUST_TYPE = RustType(
    name="crate::core::Any",
    docs="Any non-null value.",
)

PRIM_RUST_TYPES = {
    # Salad types to be overridden
    "boolean": BOOL_RUST_TYPE,
    "int": INT_RUST_TYPE,
    "long": LONG_RUST_TYPE,
    "float": FLOAT_RUST_TYPE,
    "double": DOUBLE_RUST_TYPE,
    "string": STRING_RUST_TYPE,
    "org.w3id.cwl.salad.Any": ANY_RUST_TYPE,
    "org.w3id.cwl.salad.RecordSchema.type.Record_name": UnionRustType(
        name="RecordType",
        value="record",
    ),
    "org.w3id.cwl.salad.ArraySchema.type.Array_name": UnionRustType(
        name="ArrayType",
        value="array",
    ),
    "org.w3id.cwl.salad.EnumSchema.type.Enum_name": UnionRustType(
        name="EnumType",
        value="enum",
    ),

    # CWL types to be overridden
    "org.w3id.cwl.cwl.Expression": RustType(
        name=STRING_RUST_TYPE.name,
        docs=[
            "Fragment of Javascript/ECMAScript 5.1 code evaluated by the workflow",
            "platform to affect the inputs, outputs, or behavior of a process",
        ],
        variant="Expression",
    ),
}


#
# Code generator
#


class RustCodeGen(CodeGenBase):
    def __init__(
        self,
        package: str,
        salad_version: str,
        target: Optional[str],
    ) -> None:
        self.package = package
        self.target_dir = Path(target or ".").resolve()
        self.src_dir = self.target_dir / "src"
        self.salad_version = salad_version[1:] if salad_version.startswith("v") else salad_version

        self.rust_names = RustNames(PRIM_RUST_TYPES)

    def parse(self, items: List[Dict[str, Any]]) -> None:
        # Copy template Rust sources
        template_dir = str(files("schema_salad").joinpath("rust/"))
        for root, _, files_ in os.walk(template_dir):
            for name in files_:
                src = Path(root, name)
                dst = self.target_dir / os.path.relpath(src, template_dir)

                replace = None
                if name == "Cargo.toml":
                    replace = {
                        "{package_name}": self.target_dir.name,
                        "{package_version}": f"0.1.0+salad{self.salad_version}",
                    }

                safe_file_copy(src, dst, replace)

        # Parse Avro schemas to Rust types
        avro_schemas = make_avro(items)
        salad_package = parser_info()

        for schema in avro_schemas:
            if not schema.name.startswith(salad_package):
                rust_ty = self.schema_to_rust(schema)
                self.rust_names.add(schema.name, rust_ty)
    
        # Generate the Rust source code
        with IOWrapper.open(f"{self.src_dir}/lib.rs", "a") as w:
            for rust in self.rust_names:
                rust.write_to(w)

    def schema_to_rust(self, schema: Schema) -> RustType:
        salad_attrs: Dict[str, Any] = {}
        if schema.get_prop("documentRoot") is True:
            salad_attrs["root"] = None

        # Parse record schema
        if isinstance(schema, RecordSchema):
            return StructRustType(
                name=schema.name,
                fields=[self.schema_to_field(f, schema) for f in schema.fields],
                docs=schema.get_prop("doc"),
                salad_attrs=salad_attrs,
            )

        # Parse enum schema
        if isinstance(schema, EnumSchema):
            if len(schema.symbols) == 1:
                return UnionRustType(
                    name=schema.name,
                    value=schema.symbols[0],
                    docs=schema.get_prop("doc"),
                    salad_attrs=salad_attrs,
                )

            return EnumRustType(
                name=schema.name,
                variants=[EnumUnitVariant(s) for s in schema.symbols],
                docs=schema.get_prop("doc"),
                salad_attrs=salad_attrs,
            )

        raise SchemaException(f"Failed to parse the schema of type {type(schema).__name__}.")

    def schema_to_field(self, field: SaladField, record: RecordSchema) -> StructField:
        def schema_to_field_type(schema: Schema, parent_name: str) -> Union[RustType, RustTypeRef]:
            # Parse union schema
            if isinstance(schema, UnionSchema):
                name = f"{parent_name}_{field.name}"
                schemas = [s for s in schema.schemas if s.type != "null"]
                schemas_len = len(schemas)

                # Parse one-schema unions
                if schemas_len == 1:
                    variant_ty = schema_to_field_type(schemas[0], parent_name)
                    if schemas_len < len(schema.schemas):
                        return OptionRustType(variant_ty)
                    return variant_ty

                # Parse unions with multiple schemas
                ty = self.rust_names.add(
                    name,
                    EnumRustType(
                        name=name,
                        variants=[
                            EnumTupleVariant(schema_to_field_type(v, name))
                            for v in schemas
                            if v.type != "null"
                        ],
                        docs=schema.get_prop("doc"),
                    ),
                )
                if schemas_len < len(schema.schemas):
                    return OptionRustType(ty)
                return ty

            # Parse array schema
            if isinstance(schema, ArraySchema):
                return ArrayRustType(schema_to_field_type(schema.items, parent_name))

            # Parse record schema
            if isinstance(schema, (EnumSchema, RecordSchema)):
                return self.rust_names.get_ref(schema.name)

            # Parse primitive schema
            if isinstance(schema, PrimitiveSchema):
                return self.rust_names.get_ref(schema.type)

            raise ValueError(f"Failed to parse the schema of type {type(schema).__name__}.")

        # Parse struct field
        salad_attrs: Dict[str, Any] = {}
        jsonldPred = field.get_prop("jsonldPredicate")

        if isinstance(jsonldPred, str):
            if jsonldPred == "@id":
                salad_attrs["identifier"] = None
        elif isinstance(jsonldPred, dict):
            if (map_key := jsonldPred.get("mapSubject")) is not None:
                salad_attrs["map_key"] = map_key
            if (map_predicate := jsonldPred.get("mapPredicate")) is not None:
                salad_attrs["map_predicate"] = map_predicate

        return StructField(
            name=field.name,
            ty=schema_to_field_type(field.type, record.name),
            docs=field.get_prop("doc"),
            salad_attrs=salad_attrs,
        )
