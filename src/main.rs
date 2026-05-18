use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    Json, Router, http::StatusCode
};
use rusqlite::{params, Connection};
use serde::Deserialize;
use std::net::SocketAddr;

// Estructura limpia para recibir los datos del formulario
#[derive(Deserialize)]
struct NuevaOfrenda {
    monto: f64,
    secretaria_id: i32,
}

#[tokio::main]
async fn main() {
    // Asegurar que la base de datos tenga los datos base al arrancar
    if let Ok(conn) = Connection::open("../iglesia.db") {
        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (1, 'Niños')", []);
        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (2, 'Educacional')", []);
        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (3, 'General')", []);
        let _ = conn.execute("INSERT OR IGNORE INTO tesoreros (id, nombre, correo) VALUES (1, 'Hermano Juan', 'juan@iglesia.com')", []);
    }

    // Solo dos rutas: la pantalla de inicio y la API para guardar
    let app = Router::new()
        .route("/", get(servir_pantalla_inicio))
        .route("/api/ofrendas", post(guardar_ofrenda_api));

    // Cambiamos 127.0.0.1 por 0.0.0.0 para abrirlo a la red local
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("-> Servidor corriendo en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn servir_pantalla_inicio() -> Html<&'static str> {
    Html(include_str!("index.html"))
}

// Procesar el envío del formulario
async fn guardar_ofrenda_api(Json(payload): Json<NuevaOfrenda>) -> impl IntoResponse {
    // DETECTOR 1: Esto saldrá en tu terminal de Mac apenas des clic en el botón
    println!("-> Servidor recibió: Monto = {}, Secretaría ID = {}", payload.monto, payload.secretaria_id);

    let conn = match Connection::open("../iglesia.db") {
        Ok(c) => c,
        Err(e) => {
            println!("[ERROR] No se pudo abrir iglesia.db: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Error al abrir BD".to_string());
        }
    };

    let tesorero_id = 1; // ID fijo por ahora

    // Intentar insertar en SQLite
    match conn.execute(
        "INSERT INTO ingresos_ofrendas (monto, secretaria_id, tesorero_id) VALUES (?1, ?2, ?3)",
        params![payload.monto, payload.secretaria_id, tesorero_id],
    ) {
        Ok(_) => {
            // DETECTOR 2: Si todo sale bien, verás esto
            println!("¡ÉXITO! Registro guardado correctamente en SQLite.");
            (StatusCode::OK, "Guardado exitosamente en la base de datos.".to_string())
        },
        Err(e) => {
            // DETECTOR 3: Si SQLite rechaza el dato (por ejemplo, por los triggers), aquí te dirá la razón exacta
            println!("[ERROR SQLITE] El registro fue rechazado: {}", e);
            (StatusCode::BAD_REQUEST, format!("Rechazado por la BD: {}", e))
        }
    }
}
