# Avisos de terceros

## BBOT

CHEF migra y adapta capacidades del proyecto **BBOT 3.0.2**, publicado por Black Lantern Security. La referencia usada está en `C:\projects\CHEF`; esa copia no contiene historial Git y se conserva sin modificar. Proyecto upstream: [blacklanternsecurity/bbot](https://github.com/blacklanternsecurity/bbot).

El código fuente upstream de BBOT está bajo GNU Affero General Public License, versión 3 (AGPL-3.0), y conserva esa licencia y su atribución al portarse. CHEF también adopta AGPL-3.0; por tanto, el archivo [`LICENSE`](LICENSE) contiene el texto aplicable a CHEF y al material BBOT derivado sin reemplazar los avisos de origen. Los archivos derivados deben indicar los cambios y su fecha, y acompañarse del código fuente correspondiente conforme a la licencia. El estado de la migración está documentado en CHEF.

El texto íntegro de AGPL-3.0 está en [`LICENSE`](LICENSE). El resto de dependencias conserva los avisos y condiciones indicados por sus respectivos proveedores.

## `psl` (Rust)

CHEF fija `psl` 2.1.239, con licencia MIT o Apache-2.0 y Public Suffix List integrada. La usa únicamente para derivar pistas locales de dominio padre; no realiza consultas DNS. Se conservan los avisos y textos de licencia distribuidos por el crate.

## `idna` (Rust)

CHEF utiliza `idna` 1.1.0, con licencia MIT o Apache-2.0, para convertir a Unicode los labels punycode al derivar nombres de organización. La misma versión ya estaba fijada como dependencia transitiva de `url`; se declara directamente para esta conversión local.

## `deunicode` (Rust)

CHEF fija `deunicode` 1.6.2, con licencia BSD-3-Clause, para generar el alias ASCII transliterado de los nombres Unicode derivados, siguiendo la rama equivalente de BBOT.
