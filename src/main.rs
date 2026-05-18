use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Json, Router, http::StatusCode
};
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize)]
struct NuevaOfrenda {
    monto: f64,
    secretaria_id: i32,
}

// NUEVO: Función para saber dónde guardar la BD
fn obtener_ruta_bd() -> String {
    std::env::var("DATABASE_PATH").unwrap_or_else(|_| "../iglesia.db".to_string())
}

#[tokio::main]
async fn main() {
    let db_path = obtener_ruta_bd();
    
    if let Ok(conn) = Connection::open(&db_path) {
        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (1, 'Niños')", []);
        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (2, 'Educacional')", []);
        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (3, 'General')", []);
        let _ = conn.execute("INSERT OR IGNORE INTO tesoreros (id, nombre, correo) VALUES (1, 'Hermano Juan', 'juan@iglesia.com')", []);
    }

    let app = Router::new()
        .route("/", get(servir_pantalla_inicio))
        .route("/api/ofrendas", post(guardar_ofrenda_api));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("-> Servidor corriendo en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn servir_pantalla_inicio() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

async fn guardar_ofrenda_api(Json(payload): Json<NuevaOfrenda>) -> impl IntoResponse {
    let db_path = obtener_ruta_bd();
    
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Error al abrir BD".to_string())
    };

    let tesorero_id = 1;

    match conn.execute(
        "INSERT INTO ingresos_ofrendas (monto, secretaria_id, tesorero_id) VALUES (?1, ?2, ?3)",
        params![payload.monto, payload.secretaria_id, tesorero_id],
    ) {
        Ok(_) => (StatusCode::OK, "Guardado exitosamente.".to_string()),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Rechazado por la BD: {}", e))
    }
}