//! Composición pura de capas TOML, adaptada de `core/config/merge.py` de BBOT.
//!
//! No abre archivos ni accede al entorno. Las capas ya cargadas se fusionan en
//! memoria antes de validar `ChefConfig`; las tablas se fusionan por clave y
//! arrays/escalars de una capa posterior reemplazan el valor anterior.

use toml::{Table, Value};

use crate::{ChefConfig, ChefError, ChefResult, ErrorCode};

const MAX_LAYERS: usize = 16;
const MAX_LAYER_BYTES: usize = 64 * 1024;
const MAX_TOTAL_BYTES: usize = 256 * 1024;
const MAX_MERGE_DEPTH: usize = 32;
const MAX_CONFIG_NODES: usize = 16_384;
const MAX_PATH_BYTES: usize = 1024;
const MAX_PATH_SEGMENTS: usize = 32;
const MAX_LEAF_PATHS: usize = 4096;

/// Fusiona tablas TOML recursivamente. La capa de mayor índice gana en hojas;
/// arrays se reemplazan enteros y los valores de entrada no se modifican.
pub fn deep_merge_toml(base: Option<&Value>, updates: &[Option<&Value>]) -> ChefResult<Value> {
    let mut nodes = 0;
    if let Some(base) = base {
        validate_value_tree(base, 0, &mut nodes)?;
    }
    for update in updates.iter().flatten() {
        validate_value_tree(update, 0, &mut nodes)?;
    }
    let mut result = match base {
        Some(Value::Table(table)) => Value::Table(table.clone()),
        Some(_) => {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La base de configuración CHEF debe ser una tabla TOML",
            ));
        }
        None => Value::Table(Table::new()),
    };

    for update in updates.iter().flatten() {
        let Value::Table(update) = update else {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Cada capa de configuración CHEF debe ser una tabla TOML",
            ));
        };
        merge_table(
            result.as_table_mut().expect("la raíz siempre es tabla"),
            update,
            0,
        )?;
    }
    Ok(result)
}

/// Fusiona capas TOML en orden y devuelve una configuración CHEF ya validada.
/// El primer elemento es el valor base y cada capa posterior lo sobrescribe.
pub fn parse_layered_toml(layers: &[&[u8]]) -> ChefResult<ChefConfig> {
    if layers.is_empty() || layers.len() > MAX_LAYERS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Cantidad de capas TOML CHEF fuera de límites",
        ));
    }
    let Some(total_bytes) = layers
        .iter()
        .try_fold(0_usize, |total, layer| total.checked_add(layer.len()))
    else {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Las capas TOML CHEF superan el máximo de bytes",
        ));
    };
    if total_bytes > MAX_TOTAL_BYTES || layers.iter().any(|layer| layer.len() > MAX_LAYER_BYTES) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Las capas TOML CHEF superan el máximo de bytes",
        ));
    }

    let parsed = layers
        .iter()
        .map(|bytes| {
            let text = std::str::from_utf8(bytes)
                .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "TOML CHEF debe ser UTF-8"))?;
            toml::from_str::<Value>(text)
                .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Capa TOML CHEF inválida"))
        })
        .collect::<ChefResult<Vec<_>>>()?;
    let refs = parsed.iter().map(Some).collect::<Vec<_>>();
    let merged = deep_merge_toml(None, &refs)?;
    let config: ChefConfig = merged.try_into().map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La configuración combinada no cumple el esquema CHEF",
        )
    })?;
    config.validate()?;
    Ok(config)
}

/// Busca una ruta con puntos. Las claves que contienen puntos literales no
/// tienen escape, siguiendo la semántica de BBOT.
pub fn dotted_get<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
    let parts = split_path(path).ok()?;
    let mut cursor = data;
    for part in parts {
        cursor = cursor.as_table()?.get(part)?;
    }
    Some(cursor)
}

/// Escribe una ruta con puntos; los intermediarios que no sean tablas se
/// reemplazan por tablas antes de colocar el valor final.
pub fn dotted_set(data: &mut Value, path: &str, value: Value) -> ChefResult<()> {
    let parts = split_path(path)?;
    let mut nodes = 0;
    validate_value_tree(&value, 0, &mut nodes)?;
    let mut cursor = data.as_table_mut().ok_or_else(|| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La configuración CHEF debe tener una tabla raíz",
        )
    })?;
    for part in &parts[..parts.len() - 1] {
        let entry = cursor
            .entry((*part).to_owned())
            .or_insert_with(|| Value::Table(Table::new()));
        if !entry.is_table() {
            *entry = Value::Table(Table::new());
        }
        cursor = entry.as_table_mut().expect("se normalizó el intermediario");
    }
    cursor.insert(parts[parts.len() - 1].to_owned(), value);
    Ok(())
}

/// Lista hojas con rutas separadas por puntos; las tablas vacías son hojas.
pub fn iter_dotted_paths(data: &Value) -> ChefResult<Vec<String>> {
    let mut nodes = 0;
    validate_value_tree(data, 0, &mut nodes)?;
    let root = data.as_table().ok_or_else(|| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La configuración CHEF debe tener una tabla raíz",
        )
    })?;
    let mut result = Vec::new();
    collect_paths(root, "", 0, &mut result)?;
    Ok(result)
}

fn merge_table(base: &mut Table, update: &Table, depth: usize) -> ChefResult<()> {
    if depth > MAX_MERGE_DEPTH {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Profundidad de capas TOML CHEF fuera de límites",
        ));
    }
    for (key, update_value) in update {
        match (base.get_mut(key), update_value) {
            (Some(Value::Table(base_table)), Value::Table(update_table)) => {
                merge_table(base_table, update_table, depth + 1)?;
            }
            _ => {
                base.insert(key.clone(), update_value.clone());
            }
        }
    }
    Ok(())
}

fn validate_value_tree(value: &Value, depth: usize, nodes: &mut usize) -> ChefResult<()> {
    if depth > MAX_MERGE_DEPTH {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Profundidad de configuración TOML CHEF fuera de límites",
        ));
    }
    *nodes += 1;
    if *nodes > MAX_CONFIG_NODES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Cantidad de valores TOML CHEF supera el máximo",
        ));
    }
    match value {
        Value::Table(table) => {
            for value in table.values() {
                validate_value_tree(value, depth + 1, nodes)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                validate_value_tree(value, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn split_path(path: &str) -> ChefResult<Vec<&str>> {
    if path.is_empty() || path.len() > MAX_PATH_BYTES {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Ruta de configuración CHEF vacía o demasiado larga",
        ));
    }
    let parts = path.split('.').collect::<Vec<_>>();
    if parts.len() > MAX_PATH_SEGMENTS || parts.iter().any(|part| part.is_empty()) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Ruta de configuración CHEF inválida",
        ));
    }
    Ok(parts)
}

fn collect_paths(
    table: &Table,
    prefix: &str,
    depth: usize,
    result: &mut Vec<String>,
) -> ChefResult<()> {
    if depth > MAX_MERGE_DEPTH {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Profundidad de rutas TOML CHEF fuera de límites",
        ));
    }
    for (key, value) in table {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        if path.len() > MAX_PATH_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Ruta de configuración CHEF demasiado larga",
            ));
        }
        if let Value::Table(child) = value
            && !child.is_empty()
        {
            collect_paths(child, &path, depth + 1, result)?;
        } else {
            result.push(path);
            if result.len() > MAX_LEAF_PATHS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "Cantidad de rutas CHEF supera el máximo",
                ));
            }
        }
    }
    Ok(())
}
