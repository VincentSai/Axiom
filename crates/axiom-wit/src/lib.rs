//! WebAssembly Interface Type generation from the Axiom AST.

use std::fmt;

use axiom_ast::{EffectDecl, FunctionDecl, FunctionParam, Module, Type, TypeDef, VariantDef};
use axiom_parser::{parse_module, ParseError};

/// An error encountered while generating a WIT package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitError {
    /// The Axiom source could not be parsed.
    Parse(ParseError),
    /// The requested WIT package name is empty or malformed.
    InvalidPackageName(String),
    /// An Axiom custom type cannot be represented by this generator.
    UnsupportedType(String),
}

impl fmt::Display for WitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(error) => write!(formatter, "failed to parse Axiom source: {error}"),
            Self::InvalidPackageName(name) => {
                write!(formatter, "invalid WIT package name `{name}`")
            }
            Self::UnsupportedType(ty) => {
                write!(formatter, "unsupported Axiom type `{ty}` in WIT output")
            }
        }
    }
}

impl std::error::Error for WitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(error),
            Self::InvalidPackageName(_) | Self::UnsupportedType(_) => None,
        }
    }
}

impl From<ParseError> for WitError {
    fn from(error: ParseError) -> Self {
        Self::Parse(error)
    }
}

/// Generate a WIT package from an already parsed Axiom module.
///
/// A package name without a namespace is placed in the `axiom` namespace. For
/// example, `payments` becomes `axiom:payments`. Passing a full package ID such
/// as `acme:payments` preserves the supplied namespace.
pub fn generate_wit(module: &Module, package_name: &str) -> Result<String, WitError> {
    let (package_id, world_name) = normalize_package_name(package_name)?;
    let type_names = module
        .types
        .iter()
        .map(type_definition_name)
        .map(wit_identifier)
        .collect::<Vec<_>>();

    let mut output = format!("package {package_id};\n");

    if !module.types.is_empty() {
        output.push_str("\ninterface types {\n");
        for (index, definition) in module.types.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            render_type_definition(&mut output, definition)?;
        }
        output.push_str("}\n");
    }

    for effect in &module.effects {
        output.push('\n');
        render_effect(&mut output, effect, &type_names)?;
    }

    output.push_str(&format!("\nworld {world_name} {{\n"));
    if !type_names.is_empty() {
        output.push_str("    export types;\n");
    }
    for effect in &module.effects {
        output.push_str(&format!("    import {};\n", wit_identifier(&effect.name)));
    }
    if !type_names.is_empty() && !module.functions.is_empty() {
        output.push('\n');
        render_use(&mut output, &type_names, 4);
    }
    if (!type_names.is_empty() || !module.effects.is_empty()) && !module.functions.is_empty() {
        output.push('\n');
    }
    for function in &module.functions {
        render_function_export(&mut output, function)?;
    }
    output.push_str("}\n");

    Ok(output)
}

/// Parse Axiom source and generate its WIT package.
pub fn generate_wit_from_source(source: &str, package_name: &str) -> Result<String, WitError> {
    generate_wit(&parse_module(source)?, package_name)
}

fn normalize_package_name(package_name: &str) -> Result<(String, String), WitError> {
    let package_name = package_name.trim();
    if package_name.is_empty() {
        return Err(WitError::InvalidPackageName(package_name.into()));
    }

    let (namespace, name_and_version) = match package_name.split_once(':') {
        Some((namespace, name)) if !namespace.is_empty() && !name.is_empty() => {
            if name.contains(':') {
                return Err(WitError::InvalidPackageName(package_name.into()));
            }
            (wit_identifier(namespace), name)
        }
        Some(_) => return Err(WitError::InvalidPackageName(package_name.into())),
        None => ("axiom".into(), package_name),
    };

    let (name, version) = name_and_version
        .split_once('@')
        .map_or((name_and_version, None), |(name, version)| {
            (name, Some(version))
        });
    if name.is_empty() || version.is_some_and(str::is_empty) {
        return Err(WitError::InvalidPackageName(package_name.into()));
    }

    let name = wit_identifier(name);
    let package_id = version.map_or_else(
        || format!("{namespace}:{name}"),
        |version| format!("{namespace}:{name}@{version}"),
    );
    Ok((package_id, name))
}

fn type_definition_name(definition: &TypeDef) -> &str {
    match definition {
        TypeDef::Record { name, .. } | TypeDef::Enum { name, .. } => name,
    }
}

fn render_type_definition(output: &mut String, definition: &TypeDef) -> Result<(), WitError> {
    match definition {
        TypeDef::Record { name, fields } => {
            output.push_str(&format!("    record {} {{\n", wit_identifier(name)));
            for field in fields {
                output.push_str(&format!(
                    "        {}: {},\n",
                    wit_identifier(&field.name),
                    render_type(&field.ty)?
                ));
            }
            output.push_str("    }\n");
        }
        TypeDef::Enum { name, variants } if variants.iter().all(unit_variant) => {
            output.push_str(&format!("    enum {} {{\n", wit_identifier(name)));
            for variant in variants {
                output.push_str(&format!("        {},\n", wit_identifier(&variant.name)));
            }
            output.push_str("    }\n");
        }
        TypeDef::Enum { name, variants } => {
            output.push_str(&format!("    variant {} {{\n", wit_identifier(name)));
            for variant in variants {
                render_variant(output, variant)?;
            }
            output.push_str("    }\n");
        }
    }
    Ok(())
}

fn unit_variant(variant: &VariantDef) -> bool {
    variant.fields.is_empty()
}

fn render_variant(output: &mut String, variant: &VariantDef) -> Result<(), WitError> {
    let name = wit_identifier(&variant.name);
    match variant.fields.as_slice() {
        [] => output.push_str(&format!("        {name},\n")),
        [field] => output.push_str(&format!("        {name}({}),\n", render_type(&field.ty)?)),
        fields => {
            let payload = fields
                .iter()
                .map(|field| render_type(&field.ty))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ");
            output.push_str(&format!("        {name}(tuple<{payload}>),\n"));
        }
    }
    Ok(())
}

fn render_effect(
    output: &mut String,
    effect: &EffectDecl,
    type_names: &[String],
) -> Result<(), WitError> {
    output.push_str(&format!("interface {} {{\n", wit_identifier(&effect.name)));
    if !type_names.is_empty() {
        render_use(output, type_names, 4);
        if !effect.operations.is_empty() {
            output.push('\n');
        }
    }
    for operation in &effect.operations {
        output.push_str(&format!(
            "    {}: {};\n",
            wit_identifier(&operation.name),
            render_function_type(&operation.params, &operation.return_type)?
        ));
    }
    output.push_str("}\n");
    Ok(())
}

fn render_use(output: &mut String, type_names: &[String], indentation: usize) {
    output.push_str(&" ".repeat(indentation));
    output.push_str("use types.{");
    output.push_str(&type_names.join(", "));
    output.push_str("};\n");
}

fn render_function_export(output: &mut String, function: &FunctionDecl) -> Result<(), WitError> {
    output.push_str(&format!(
        "    export {}: {};\n",
        wit_identifier(&function.name),
        render_function_type(&function.params, &function.return_type)?
    ));
    Ok(())
}

fn render_function_type(params: &[FunctionParam], return_type: &Type) -> Result<String, WitError> {
    let params = params
        .iter()
        .map(|param| {
            Ok(format!(
                "{}: {}",
                wit_identifier(&param.name),
                render_type(&param.ty)?
            ))
        })
        .collect::<Result<Vec<_>, WitError>>()?
        .join(", ");
    Ok(format!("func({params}) -> {}", render_type(return_type)?))
}

fn render_type(ty: &Type) -> Result<String, WitError> {
    match ty {
        Type::Int => Ok("s64".into()),
        Type::Float => Ok("f64".into()),
        Type::Decimal | Type::String => Ok("string".into()),
        Type::Bool => Ok("bool".into()),
        Type::Unit => Ok("()".into()),
        Type::Custom(name) => render_custom_type(name),
    }
}

fn render_custom_type(name: &str) -> Result<String, WitError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WitError::UnsupportedType(name.into()));
    }

    let Some(open) = name.find('<') else {
        return Ok(match name {
            "Int" => "s64".into(),
            "Float" => "f64".into(),
            "Decimal" | "String" => "string".into(),
            "Bool" => "bool".into(),
            "Unit" | "()" => "()".into(),
            custom => wit_identifier(custom),
        });
    };
    if !name.ends_with('>') {
        return Err(WitError::UnsupportedType(name.into()));
    }

    let constructor = name[..open].trim();
    let arguments = split_type_arguments(&name[open + 1..name.len() - 1])?;
    let rendered = arguments
        .iter()
        .map(|argument| render_custom_type(argument))
        .collect::<Result<Vec<_>, _>>()?;
    let expected_arity = match constructor {
        "List" | "Option" => 1,
        "Result" => 2,
        _ => return Err(WitError::UnsupportedType(name.into())),
    };
    if rendered.len() != expected_arity {
        return Err(WitError::UnsupportedType(name.into()));
    }

    Ok(format!(
        "{}<{}>",
        constructor.to_ascii_lowercase(),
        rendered.join(", ")
    ))
}

fn split_type_arguments(arguments: &str) -> Result<Vec<&str>, WitError> {
    let mut result = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, character) in arguments.char_indices() {
        match character {
            '<' => depth += 1,
            '>' => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| WitError::UnsupportedType(arguments.into()))?;
            }
            ',' if depth == 0 => {
                result.push(arguments[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(WitError::UnsupportedType(arguments.into()));
    }
    result.push(arguments[start..].trim());
    if result.iter().any(|argument| argument.is_empty()) {
        return Err(WitError::UnsupportedType(arguments.into()));
    }
    Ok(result)
}

fn wit_identifier(name: &str) -> String {
    let characters = name.chars().collect::<Vec<_>>();
    let mut output = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        if character == '_' || character == '-' || character.is_whitespace() {
            if !output.is_empty() && !output.ends_with('-') {
                output.push('-');
            }
            continue;
        }

        if character.is_ascii_uppercase() {
            let previous_is_lower_or_digit = index > 0
                && (characters[index - 1].is_ascii_lowercase()
                    || characters[index - 1].is_ascii_digit());
            let acronym_boundary = index > 0
                && characters[index - 1].is_ascii_uppercase()
                && characters
                    .get(index + 1)
                    .is_some_and(char::is_ascii_lowercase);
            if (previous_is_lower_or_digit || acronym_boundary)
                && !output.is_empty()
                && !output.ends_with('-')
            {
                output.push('-');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character.to_ascii_lowercase());
        }
    }
    output.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::{render_custom_type, wit_identifier};

    #[test]
    fn converts_axiom_names_to_wit_identifiers() {
        assert_eq!(wit_identifier("AuditLog"), "audit-log");
        assert_eq!(wit_identifier("HTTPServer"), "http-server");
        assert_eq!(wit_identifier("execute_transfer"), "execute-transfer");
    }

    #[test]
    fn converts_nested_axiom_container_types() {
        assert_eq!(
            render_custom_type("Result<Option<Account>, List<String>>").unwrap(),
            "result<option<account>, list<string>>"
        );
    }
}
