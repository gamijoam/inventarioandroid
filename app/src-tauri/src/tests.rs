use rusqlite::Connection;

use crate::{
    auth, currency, db, error::AppError, products, products::ProductInput,
};

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    db::init(&c).unwrap();
    c
}

fn admin_id(c: &Connection) -> String {
    auth::login(c, "admin", "admin123").unwrap().id
}

fn input(sku: &str, nombre: &str) -> ProductInput {
    ProductInput {
        sku: sku.into(),
        nombre: nombre.into(),
        descripcion: String::new(),
        moneda: None,
        precio: "10,50".into(),
        costo: "7".into(),
        stock: 3,
    }
}

// ---------- Autenticación ----------

#[test]
fn login_con_admin_por_defecto() {
    let c = conn();
    let u = auth::login(&c, "admin", "admin123").unwrap();
    assert_eq!(u.username, "admin");
    assert!(matches!(auth::login(&c, "admin", "mala"), Err(AppError::InvalidCredentials)));
    assert!(matches!(auth::login(&c, "nadie", "admin123"), Err(AppError::InvalidCredentials)));
}

#[test]
fn init_es_idempotente() {
    let c = conn();
    db::init(&c).unwrap();
    let n: i64 = c.query_row("SELECT COUNT(*) FROM usuarios", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);
    assert_eq!(db::schema_version(&c).unwrap(), 2);
}

#[test]
fn cambiar_contrasena() {
    let c = conn();
    let id = admin_id(&c);
    assert!(auth::change_password(&c, &id, "mala", "nueva123").is_err());
    auth::change_password(&c, &id, "admin123", "nueva123").unwrap();
    assert!(auth::login(&c, "admin", "nueva123").is_ok());
    assert!(auth::login(&c, "admin", "admin123").is_err());
}

// ---------- Migración desde v0.1 ----------

#[test]
fn migra_base_de_datos_v0_1_conservando_productos() {
    let c = Connection::open_in_memory().unwrap();
    // Esquema exacto de la v0.1 (sin user_version).
    c.execute_batch(
        r#"
        CREATE TABLE usuarios (id TEXT PRIMARY KEY, username TEXT NOT NULL UNIQUE COLLATE NOCASE,
            nombre TEXT NOT NULL, password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')));
        CREATE TABLE productos (id TEXT PRIMARY KEY, sku TEXT NOT NULL UNIQUE COLLATE NOCASE,
            nombre TEXT NOT NULL, descripcion TEXT NOT NULL DEFAULT '',
            precio REAL NOT NULL DEFAULT 0 CHECK (precio >= 0),
            stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')));
        INSERT INTO productos (id, sku, nombre, precio, stock) VALUES ('p1', 'A-1', 'Martillo', 12.5, 20);
        INSERT INTO productos (id, sku, nombre, precio, stock) VALUES ('p2', 'B-2', 'Clavo', 0.1, 500);
        "#,
    )
    .unwrap();
    db::init(&c).unwrap();

    assert_eq!(db::schema_version(&c).unwrap(), 2);
    let p = products::get(&c, "p1").unwrap();
    assert_eq!((p.moneda.as_str(), p.precio, p.costo, p.stock), ("USD", 1250, 0, 20));
    assert_eq!(products::get(&c, "p2").unwrap().precio, 10);
}

// ---------- Productos ----------

#[test]
fn crud_de_productos() {
    let c = conn();
    let p = products::create(&c, input("A-1", "Martillo")).unwrap();
    assert_eq!((p.moneda.as_str(), p.precio, p.costo), ("USD", 1050, 700));
    products::create(&c, input("B-2", "Clavos")).unwrap();

    assert_eq!(products::list(&c, None).unwrap().len(), 2);
    assert_eq!(products::list(&c, Some("mart")).unwrap().len(), 1);
    assert_eq!(products::list(&c, Some("b-2")).unwrap().len(), 1);

    let mut cambio = input("A-1", "Martillo grande");
    cambio.stock = 9;
    cambio.precio = "15".into();
    let p2 = products::update(&c, &p.id, cambio).unwrap();
    assert_eq!((p2.nombre.as_str(), p2.stock, p2.precio), ("Martillo grande", 9, 1500));

    products::delete(&c, &p.id).unwrap();
    assert!(matches!(products::get(&c, &p.id), Err(AppError::NotFound)));
    assert!(matches!(products::delete(&c, &p.id), Err(AppError::NotFound)));
}

#[test]
fn validaciones_de_producto() {
    let c = conn();
    products::create(&c, input("X", "Uno")).unwrap();
    let invalido = |i: ProductInput| matches!(products::create(&c, i), Err(AppError::Validation(_)));
    assert!(invalido(input("x", "Dos")), "SKU duplicado");
    assert!(invalido(input("", "Sin sku")));
    assert!(invalido(input("Y", "  ")));
    let mut neg = input("Z", "Negativo");
    neg.stock = -1;
    assert!(invalido(neg));
    let mut decimales = input("Z", "Muchos decimales");
    decimales.precio = "1,999".into();
    assert!(invalido(decimales));
    let mut inactiva = input("Z", "Moneda inactiva");
    inactiva.moneda = Some("VES".into());
    assert!(invalido(inactiva), "VES no está activa por defecto");
}

// ---------- Multimoneda ----------

/// Configura el caso Venezuela: base USD, VES activa, tasa BCV 36,50.
fn venezuela(c: &Connection) -> i64 {
    currency::set_moneda_activa(c, "VES", true).unwrap();
    currency::renombrar_tipo_tasa(c, 1, "BCV").unwrap();
    currency::registrar_tasa(c, "VES", 1, "36,50", &admin_id(c)).unwrap();
    1
}

#[test]
fn precios_en_usd_con_equivalente_en_bs() {
    let c = conn();
    venezuela(&c);
    let mut i = input("M-1", "Martillo");
    i.precio = "12,50".into();
    let p = products::create(&c, i).unwrap();
    assert_eq!(p.equivalentes.len(), 1);
    assert_eq!((p.equivalentes[0].moneda.as_str(), p.equivalentes[0].precio), ("VES", 45625));

    // Un producto con precio en Bs muestra su equivalente en USD.
    let mut i = input("H-1", "Harina");
    i.moneda = Some("VES".into());
    i.precio = "73".into();
    let h = products::create(&c, i).unwrap();
    assert_eq!((h.equivalentes[0].moneda.as_str(), h.equivalentes[0].precio), ("USD", 200));
}

#[test]
fn la_tasa_mas_reciente_es_la_vigente() {
    let c = conn();
    let tipo = venezuela(&c);
    currency::registrar_tasa(&c, "VES", tipo, "40", &admin_id(&c)).unwrap();
    let vig = currency::tasas_vigentes(&c).unwrap();
    assert_eq!(vig.len(), 1);
    assert_eq!(vig[0].tasa.as_ref().unwrap().valor, "40");
    assert!(!vig[0].desactualizada);
    assert_eq!(currency::historial(&c, "VES", tipo, 30).unwrap().len(), 2);

    let p = products::create(&c, input("A", "A")).unwrap(); // 10,50 USD
    assert_eq!(p.equivalentes[0].precio, 42000); // 420,00 Bs
}

#[test]
fn tasa_de_ayer_esta_desactualizada() {
    let c = conn();
    venezuela(&c);
    c.execute("UPDATE tasas SET fecha = date('now', '-1 day')", []).unwrap();
    assert!(currency::tasas_vigentes(&c).unwrap()[0].desactualizada);
}

#[test]
fn varios_tipos_de_tasa_usa_el_por_defecto() {
    let c = conn();
    let bcv = venezuela(&c);
    let paralelo = currency::crear_tipo_tasa(&c, "Paralelo").unwrap();
    currency::registrar_tasa(&c, "VES", paralelo.id, "50", &admin_id(&c)).unwrap();

    let p = products::create(&c, input("A", "A")).unwrap();
    assert_eq!(p.equivalentes[0].precio, 38325); // 10,50 * 36,50 (BCV)

    currency::set_tipo_tasa_defecto(&c, paralelo.id).unwrap();
    assert_eq!(products::get(&c, &p.id).unwrap().equivalentes[0].precio, 52500); // * 50

    assert!(currency::eliminar_tipo_tasa(&c, paralelo.id).is_err(), "es el por defecto");
    currency::set_tipo_tasa_defecto(&c, bcv).unwrap();
    currency::eliminar_tipo_tasa(&c, paralelo.id).unwrap();
    assert_eq!(currency::tasas_vigentes(&c).unwrap().len(), 1);
}

#[test]
fn sin_tasa_no_hay_equivalente() {
    let c = conn();
    currency::set_moneda_activa(&c, "COP", true).unwrap();
    let p = products::create(&c, input("A", "A")).unwrap();
    assert!(p.equivalentes.is_empty());
    let vig = currency::tasas_vigentes(&c).unwrap();
    assert!(vig[0].tasa.is_none() && vig[0].desactualizada);
}

#[test]
fn reglas_de_monedas() {
    let c = conn();
    let id = admin_id(&c);
    assert!(currency::set_moneda_activa(&c, "USD", false).is_err(), "base");
    assert!(currency::set_moneda_activa(&c, "XYZ", true).is_err());
    assert!(currency::registrar_tasa(&c, "USD", 1, "2", &id).is_err(), "base vale 1");
    assert!(currency::registrar_tasa(&c, "VES", 1, "0", &id).is_err());
    assert!(currency::registrar_tasa(&c, "VES", 1, "-5", &id).is_err());
    assert!(currency::registrar_tasa(&c, "VES", 99, "5", &id).is_err());
    assert!(currency::crear_tipo_tasa(&c, "oficial").is_err(), "duplicado sin importar mayúsculas");

    // No se puede desactivar una moneda en uso.
    currency::set_moneda_activa(&c, "VES", true).unwrap();
    let mut i = input("A", "A");
    i.moneda = Some("VES".into());
    products::create(&c, i).unwrap();
    assert!(currency::set_moneda_activa(&c, "VES", false).is_err());

    // Cambiar la base la activa automáticamente.
    currency::set_moneda_base(&c, "COP").unwrap();
    let cfg = currency::config(&c).unwrap();
    assert_eq!(cfg.moneda_base, "COP");
    assert!(cfg.monedas.iter().any(|m| m.codigo == "COP" && m.activa));
}

#[test]
fn cop_sin_decimales() {
    let c = conn();
    currency::set_moneda_activa(&c, "COP", true).unwrap();
    currency::registrar_tasa(&c, "COP", 1, "4000", &admin_id(&c)).unwrap();
    let mut i = input("A", "A");
    i.moneda = Some("COP".into());
    i.precio = "42000".into();
    i.costo = "".into();
    let p = products::create(&c, i).unwrap();
    assert_eq!((p.precio, p.costo), (42000, 0));
    assert_eq!(p.equivalentes[0].precio, 1050); // 10,50 USD
}
