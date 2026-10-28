//! Adaptador local CHEF para cargar preferencias TOML desde el directorio de
//! configuración elegido por Tauri. No lee YAML BBOT ni archivos de secretos.

use std::{
    fs::File,
    io::{ErrorKind, Read},
    path::Path,
};

use chef_core::{ChefConfig, parse_layered_toml};

const MAX_CONFIG_BYTES: u64 = 64 * 1024;
const DEFAULT_CONFIG: &[u8] = b"";

pub fn load_config(path: &Path) -> Result<ChefConfig, String> {
    let Some(user_config) = read_optional_config(path)? else {
        return parse_layered_toml(&[DEFAULT_CONFIG])
            .map_err(|_| "La configuración predeterminada de CHEF no es válida".to_owned());
    };

    parse_layered_toml(&[DEFAULT_CONFIG, user_config.as_slice()])
        .map_err(|_| "El archivo config.toml de CHEF no es válido".to_owned())
}

fn read_optional_config(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("No se pudo leer la configuración local de CHEF".to_owned()),
    };

    let mut contents = Vec::new();
    file.take(MAX_CONFIG_BYTES + 1)
        .read_to_end(&mut contents)
        .map_err(|_| "No se pudo leer la configuración local de CHEF".to_owned())?;
    if contents.len() as u64 > MAX_CONFIG_BYTES {
        return Err("El archivo config.toml de CHEF supera el máximo de 64 KiB".to_owned());
    }
    Ok(Some(contents))
}
