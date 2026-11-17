use rusqlite::Connection;

use crate::{auth, db, error::AppError, products, products::ProductInput};

fn conn() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    db::init(&c).unwrap();
    c
}

fn input(sku: &str, nombre: &str) -> ProductInput {
    ProductInput {
        sku: sku.into(),
        nombre: nombre.into(),
        descripcion: String::new(),
        precio: 10.5,
        stock: 3,
    }
}

#[test]
fn login_con_admin_por_defecto() {
    let c = conn();
    let u = auth::login(&c, "admin", "admin123").unwrap();
    assert_eq!(u.username, "admin");
    assert!(matches!(auth::login(&c, "admin", "mala"), Err(AppError::InvalidCredentials)));
    assert!(matches!(auth::login(&c, "nadie", "admin123"), Err(AppError::InvalidCredentials)));
}

#[test]
fn seed_no_se_duplica() {
    let c = conn();
    db::init(&c).unwrap();
    let n: i64 = c.query_row("SELECT COUNT(*) FROM usuarios", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 1);
}

#[test]
fn cambiar_contrasena() {
    let c = conn();
    let u = auth::login(&c, "admin", "admin123").unwrap();
    assert!(auth::change_password(&c, &u.id, "mala", "nueva123").is_err());
    auth::change_password(&c, &u.id, "admin123", "nueva123").unwrap();
    assert!(auth::login(&c, "admin", "nueva123").is_ok());
    assert!(auth::login(&c, "admin", "admin123").is_err());
}

#[test]
fn crud_de_productos() {
    let c = conn();
    let p = products::create(&c, input("A-1", "Martillo")).unwrap();
    products::create(&c, input("B-2", "Clavos")).unwrap();

    assert_eq!(products::list(&c, None).unwrap().len(), 2);
    assert_eq!(products::list(&c, Some("mart")).unwrap().len(), 1);
    assert_eq!(products::list(&c, Some("b-2")).unwrap().len(), 1);

    let mut cambio = input("A-1", "Martillo grande");
    cambio.stock = 9;
    let p2 = products::update(&c, &p.id, cambio).unwrap();
    assert_eq!(p2.nombre, "Martillo grande");
    assert_eq!(p2.stock, 9);

    products::delete(&c, &p.id).unwrap();
    assert!(matches!(products::get(&c, &p.id), Err(AppError::NotFound)));
    assert!(matches!(products::delete(&c, &p.id), Err(AppError::NotFound)));
}

#[test]
fn validaciones_de_producto() {
    let c = conn();
    products::create(&c, input("X", "Uno")).unwrap();
    assert!(matches!(products::create(&c, input("x", "Dos")), Err(AppError::Validation(_))));
    assert!(matches!(products::create(&c, input("", "Sin sku")), Err(AppError::Validation(_))));
    assert!(matches!(products::create(&c, input("Y", "  ")), Err(AppError::Validation(_))));
    let mut neg = input("Z", "Negativo");
    neg.stock = -1;
    assert!(matches!(products::create(&c, neg), Err(AppError::Validation(_))));
}
