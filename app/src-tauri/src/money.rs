//! Montos exactos: se guardan como enteros en la unidad mínima de cada moneda
//! (centavos para USD, céntimos para VES, pesos para COP/CLP que no usan decimales).

use std::str::FromStr;

use rust_decimal::{Decimal, RoundingStrategy};

use crate::error::{AppError, AppResult};

/// Convierte el texto que escribe el usuario ("12,50" o "12.50") en un decimal.
pub fn parse_decimal(input: &str, campo: &str) -> AppResult<Decimal> {
    let s = input.trim().replace(' ', "");
    if s.is_empty() {
        return Ok(Decimal::ZERO);
    }
    if s.contains(',') && s.contains('.') {
        return Err(AppError::Validation(format!(
            "{campo}: usa un solo separador decimal (sin separador de miles)"
        )));
    }
    Decimal::from_str(&s.replace(',', "."))
        .map_err(|_| AppError::Validation(format!("{campo}: \"{input}\" no es un número válido")))
}

/// Texto → unidad mínima, validando que no tenga más decimales de los que admite la moneda.
pub fn parse_amount(input: &str, decimales: u32, campo: &str, moneda: &str) -> AppResult<i64> {
    let d = parse_decimal(input, campo)?.normalize();
    if d.is_sign_negative() {
        return Err(AppError::Validation(format!("{campo} no puede ser negativo")));
    }
    if d.scale() > decimales {
        return Err(AppError::Validation(if decimales == 0 {
            format!("{campo}: {moneda} no usa decimales")
        } else {
            format!("{campo}: {moneda} admite máximo {decimales} decimales")
        }));
    }
    to_minor(d, decimales)
        .ok_or_else(|| AppError::Validation(format!("{campo}: el monto es demasiado grande")))
}

/// Decimal → unidad mínima, redondeando al decimal más cercano (0,5 hacia arriba).
pub fn to_minor(value: Decimal, decimales: u32) -> Option<i64> {
    let rounded = value.round_dp_with_strategy(decimales, RoundingStrategy::MidpointAwayFromZero);
    let scaled = rounded.checked_mul(Decimal::from(10i64.checked_pow(decimales)?))?;
    i64::try_from(scaled.trunc()).ok()
}

pub fn from_minor(minor: i64, decimales: u32) -> Decimal {
    Decimal::new(minor, decimales)
}

/// Convierte un monto usando tasas expresadas contra la moneda base
/// (`tasa` = cuántas unidades de esa moneda vale 1 unidad de la base; la base vale 1).
pub fn convert(
    minor: i64,
    dec_origen: u32,
    tasa_origen: Decimal,
    dec_destino: u32,
    tasa_destino: Decimal,
) -> Option<i64> {
    if tasa_origen.is_zero() {
        return None;
    }
    let en_base = from_minor(minor, dec_origen).checked_div(tasa_origen)?;
    to_minor(en_base.checked_mul(tasa_destino)?, dec_destino)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn parsea_montos() {
        assert_eq!(parse_amount("12,50", 2, "Precio", "USD").unwrap(), 1250);
        assert_eq!(parse_amount("12.5", 2, "Precio", "USD").unwrap(), 1250);
        assert_eq!(parse_amount("", 2, "Precio", "USD").unwrap(), 0);
        assert_eq!(parse_amount("1500", 0, "Precio", "COP").unwrap(), 1500);
        assert!(parse_amount("1500,5", 0, "Precio", "COP").is_err());
        assert!(parse_amount("1,005", 2, "Precio", "USD").is_err());
        assert!(parse_amount("1.234,56", 2, "Precio", "USD").is_err());
        assert!(parse_amount("-3", 2, "Precio", "USD").is_err());
        assert!(parse_amount("abc", 2, "Precio", "USD").is_err());
    }

    #[test]
    fn convierte_usd_a_bs() {
        // 12,50 USD a 36,5 Bs/USD = 456,25 Bs
        assert_eq!(convert(1250, 2, Decimal::ONE, 2, dec("36.5")), Some(45625));
        // y de vuelta
        assert_eq!(convert(45625, 2, dec("36.5"), 2, Decimal::ONE), Some(1250));
    }

    #[test]
    fn convierte_entre_monedas_no_base_y_redondea() {
        // Base USD. 1 USD = 4000 COP, 1 USD = 36.5 VES. 10.000 COP -> 2,5 USD -> 91,25 VES
        assert_eq!(convert(10000, 0, dec("4000"), 2, dec("36.5")), Some(9125));
        // 1 USD -> COP con tasa 3999.6 = 3999.6 -> 4000 (COP sin decimales)
        assert_eq!(convert(100, 2, Decimal::ONE, 0, dec("3999.6")), Some(4000));
    }
}
