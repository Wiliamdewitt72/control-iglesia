use axum::{
    routing::{get, post},
    response::{Html, IntoResponse},
    extract::Query,
    Json, Router, http::StatusCode
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
// IMPORTANTE: Agregamos la librería de encriptación
use bcrypt::{hash, verify, DEFAULT_COST};

#[derive(Deserialize)] struct Credenciales { nombre: String, pin: String }
#[derive(Serialize)] struct SecretariaPermitida { id: i32, nombre: String, puede_autorizar: bool }
#[derive(Serialize)] struct RespuestaLogin { tesorero_id: i32, nombre: String, es_pastor: bool, secretarias: Vec<SecretariaPermitida> }

#[derive(Deserialize)] struct NuevaOfrenda { monto: f64, motivo: String, secretaria_id: i32, tesorero_id: i32, pin: String }
#[derive(Deserialize)] struct ParametrosHistorial { tesorero_id: i32, pin: String }
#[derive(Serialize)] struct RegistroHistorial { id: i32, monto: f64, motivo: String, secretaria: String, tesorero: String, fecha: String }

#[derive(Deserialize)] struct NuevaSolicitud { monto: f64, motivo: String, secretaria_id: i32, tesorero_id: i32, pin: String }
#[derive(Deserialize)] struct AprobarTicket { ticket_id: i32, tesorero_id: i32, pin: String }
#[derive(Serialize)] struct TicketEgreso { id: i32, monto: f64, motivo: String, secretaria: String, solicitante: String, estado: String, fecha: String }

fn obtener_ruta_bd() -> String { std::env::var("DATABASE_PATH").unwrap_or_else(|_| "../iglesia.db".to_string()) }

// --- FUNCIÓN DE UTILIDAD PARA VERIFICAR PINS CIFRADOS ---
fn validar_pin_seguro(conn: &Connection, tesorero_id: i32, pin_intento: &str) -> bool {
    let hash_db: Result<String, _> = conn.query_row("SELECT pin FROM tesoreros WHERE id = ?1", params![tesorero_id], |row| row.get(0));
    match hash_db {
        Ok(hash) => verify(pin_intento, &hash).unwrap_or(false), // Compara el intento con el hash
        Err(_) => false,
    }
}

#[tokio::main]
async fn main() {
    let db_path = obtener_ruta_bd();
    
    if let Ok(conn) = Connection::open(&db_path) {
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS secretarias (id INTEGER PRIMARY KEY, nombre_secretaria TEXT UNIQUE)", []);
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS tesoreros (id INTEGER PRIMARY KEY, nombre TEXT UNIQUE, pin TEXT, es_pastor BOOLEAN DEFAULT 0)", []);
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS ingresos_ofrendas (id INTEGER PRIMARY KEY AUTOINCREMENT, monto REAL, motivo TEXT, secretaria_id INTEGER, tesorero_id INTEGER, fecha_creacion TEXT DEFAULT CURRENT_TIMESTAMP)", []);
        let _ = conn.execute("CREATE TRIGGER IF NOT EXISTS bloquear_edicion_ingresos BEFORE UPDATE ON ingresos_ofrendas BEGIN SELECT RAISE(FAIL, 'No editable'); END;", []);
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS tesoreros_secretarias (tesorero_id INTEGER, secretaria_id INTEGER, puede_autorizar BOOLEAN DEFAULT 0, UNIQUE(tesorero_id, secretaria_id), FOREIGN KEY(tesorero_id) REFERENCES tesoreros(id), FOREIGN KEY(secretaria_id) REFERENCES secretarias(id))", []);
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS solicitudes_egresos (id INTEGER PRIMARY KEY AUTOINCREMENT, monto REAL NOT NULL, motivo TEXT NOT NULL, secretaria_id INTEGER NOT NULL, solicitante_id INTEGER NOT NULL, autorizador_id INTEGER, estado TEXT DEFAULT 'PENDIENTE', fecha_solicitud TEXT DEFAULT CURRENT_TIMESTAMP, fecha_autorizacion TEXT, FOREIGN KEY(secretaria_id) REFERENCES secretarias(id), FOREIGN KEY(solicitante_id) REFERENCES tesoreros(id), FOREIGN KEY(autorizador_id) REFERENCES tesoreros(id))", []);
        let _ = conn.execute("CREATE TRIGGER IF NOT EXISTS bloquear_borrado_egresos BEFORE DELETE ON solicitudes_egresos BEGIN SELECT RAISE(FAIL, 'No se pueden borrar tickets.'); END;", []);

        let _ = conn.execute("INSERT OR IGNORE INTO secretarias (id, nombre_secretaria) VALUES (1, 'Niños'), (2, 'Educacional'), (3, 'General'), (4, 'Varones')", []);
        
        // --- AQUÍ ENCRIPTAMOS LOS PINES INICIALES (SEMILLAS) ANTES DE GUARDARLOS ---
        // Ejemplo de contraseñas: Susana2026!, JuanRios8*, PastorJacobo1#
        let hash_susana = hash("Susana2026!", DEFAULT_COST).unwrap();
        let hash_juan = hash("JuanRios8*", DEFAULT_COST).unwrap();
        let hash_jacobo = hash("Jacobito1.", DEFAULT_COST).unwrap();

        let _ = conn.execute("INSERT OR IGNORE INTO tesoreros (id, nombre, pin, es_pastor) VALUES 
            (1, 'Susana Frias', ?1, 0), 
            (2, 'Juan Rios', ?2, 0), 
            (3, 'Jacobo Espinosa', ?3, 1)", 
            params![hash_susana, hash_juan, hash_jacobo]);

        let _ = conn.execute("INSERT OR IGNORE INTO tesoreros_secretarias (tesorero_id, secretaria_id, puede_autorizar) VALUES (1, 1, 0), (1, 2, 0), (2, 3, 1), (2, 4, 1), (3, 1, 1), (3, 2, 1), (3, 3, 1), (3, 4, 1)", []);
    }

    let app = Router::new()
        .route("/", get(servir_pantalla_inicio))
        .route("/api/login", post(login_api))
        .route("/api/ofrendas", post(guardar_ofrenda_api))
        .route("/api/historial", get(obtener_historial_api))
        .route("/api/solicitudes", post(crear_solicitud_api).get(listar_solicitudes_api))
        .route("/api/aprobar", post(aprobar_solicitud_api))
        .route("/api/rechazar", post(rechazar_solicitud_api));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("-> Servidor Financiero BLINDADO corriendo en http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn servir_pantalla_inicio() -> Html<&'static str> { Html(include_str!("index.html")) }

// --- LOGIN SEGURO ---
async fn login_api(Json(creds): Json<Credenciales>) -> impl IntoResponse {
    let conn = Connection::open(&obtener_ruta_bd()).unwrap();
    
    // Obtenemos el Hash guardado en la BD
    let tesorero = conn.query_row("SELECT id, pin, es_pastor FROM tesoreros WHERE nombre = ?1 COLLATE NOCASE", params![creds.nombre.trim()], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?)));
    
    match tesorero {
        Ok((t_id, pin_db, es_past)) => {
            // Comparamos el PIN plano (ej. "1234") contra el Hash (ej. "$2b$12...")
            if verify(&creds.pin, &pin_db).unwrap_or(false) {
                let mut stmt = conn.prepare("SELECT s.id, s.nombre_secretaria, ts.puede_autorizar FROM secretarias s JOIN tesoreros_secretarias ts ON s.id = ts.secretaria_id WHERE ts.tesorero_id = ?1").unwrap();
                let sec_iter = stmt.query_map([t_id], |row| Ok(SecretariaPermitida { id: row.get(0)?, nombre: row.get(1)?, puede_autorizar: row.get(2)? })).unwrap();
                let mut lista_secretarias = Vec::new();
                for s in sec_iter { lista_secretarias.push(s.unwrap()); }
                (StatusCode::OK, Json(RespuestaLogin { tesorero_id: t_id, nombre: creds.nombre.trim().to_string(), es_pastor: es_past, secretarias: lista_secretarias })).into_response()
            } else {
                (StatusCode::UNAUTHORIZED, "Credenciales inválidas".to_string()).into_response()
            }
        },
        _ => (StatusCode::UNAUTHORIZED, "Credenciales inválidas".to_string()).into_response()
    }
}

// --- TODAS LAS DEMÁS RUTAS USAN `validar_pin_seguro` ---
async fn guardar_ofrenda_api(Json(payload): Json<NuevaOfrenda>) -> impl IntoResponse {
    let conn = Connection::open(&obtener_ruta_bd()).unwrap();
    if !validar_pin_seguro(&conn, payload.tesorero_id, &payload.pin) { return (StatusCode::UNAUTHORIZED, "Denegado.".to_string()); }

    match conn.execute("INSERT INTO ingresos_ofrendas (monto, motivo, secretaria_id, tesorero_id) VALUES (?1, ?2, ?3, ?4)", params![payload.monto, payload.motivo, payload.secretaria_id, payload.tesorero_id]) {
        Ok(_) => (StatusCode::OK, "Guardado exitoso.".to_string()), Err(e) => (StatusCode::BAD_REQUEST, format!("Error: {}", e))
    }
}

async fn obtener_historial_api(Query(params): Query<ParametrosHistorial>) -> impl IntoResponse {
    let conn = match Connection::open(&obtener_ruta_bd()) { Ok(c) => c, Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json::<Vec<RegistroHistorial>>(vec![])).into_response() };
    if !validar_pin_seguro(&conn, params.tesorero_id, &params.pin) { return (StatusCode::UNAUTHORIZED, Json::<Vec<RegistroHistorial>>(vec![])).into_response(); }

    let mut stmt = conn.prepare("SELECT io.id, io.monto, COALESCE(io.motivo, ''), s.nombre_secretaria, t.nombre, io.fecha_creacion FROM ingresos_ofrendas io JOIN secretarias s ON io.secretaria_id = s.id JOIN tesoreros t ON io.tesorero_id = t.id WHERE io.secretaria_id IN (SELECT secretaria_id FROM tesoreros_secretarias WHERE tesorero_id = ?1) OR (SELECT es_pastor FROM tesoreros WHERE id = ?1) = 1 ORDER BY io.id DESC LIMIT 1000").unwrap();
    let registros_iter = stmt.query_map([params.tesorero_id], |row| { Ok(RegistroHistorial { id: row.get(0)?, monto: row.get(1)?, motivo: row.get(2)?, secretaria: row.get(3)?, tesorero: row.get(4)?, fecha: row.get(5)? }) }).unwrap();
    let mut historial = Vec::new();
    for r in registros_iter { historial.push(r.unwrap()); }
    (StatusCode::OK, Json(historial)).into_response()
}

async fn crear_solicitud_api(Json(payload): Json<NuevaSolicitud>) -> impl IntoResponse {
    let conn = Connection::open(&obtener_ruta_bd()).unwrap();
    if !validar_pin_seguro(&conn, payload.tesorero_id, &payload.pin) { return (StatusCode::UNAUTHORIZED, "Denegado.".to_string()); }

    match conn.execute("INSERT INTO solicitudes_egresos (monto, motivo, secretaria_id, solicitante_id) VALUES (?1, ?2, ?3, ?4)", params![payload.monto, payload.motivo, payload.secretaria_id, payload.tesorero_id]) {
        Ok(_) => (StatusCode::OK, "Solicitud enviada.".to_string()), Err(e) => (StatusCode::BAD_REQUEST, format!("Error BD: {}", e))
    }
}

async fn listar_solicitudes_api(Query(params): Query<ParametrosHistorial>) -> impl IntoResponse {
    let conn = match Connection::open(&obtener_ruta_bd()) { Ok(c) => c, Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json::<Vec<TicketEgreso>>(vec![])).into_response() };
    if !validar_pin_seguro(&conn, params.tesorero_id, &params.pin) { return (StatusCode::UNAUTHORIZED, Json::<Vec<TicketEgreso>>(vec![])).into_response(); }

    let mut stmt = conn.prepare("SELECT se.id, se.monto, se.motivo, s.nombre_secretaria, t.nombre, se.estado, se.fecha_solicitud FROM solicitudes_egresos se JOIN secretarias s ON se.secretaria_id = s.id JOIN tesoreros t ON se.solicitante_id = t.id WHERE se.secretaria_id IN (SELECT secretaria_id FROM tesoreros_secretarias WHERE tesorero_id = ?1) OR (SELECT es_pastor FROM tesoreros WHERE id = ?1) = 1 ORDER BY se.id DESC LIMIT 50").unwrap();
    let registros_iter = stmt.query_map([params.tesorero_id], |row| { Ok(TicketEgreso { id: row.get(0)?, monto: row.get(1)?, motivo: row.get(2)?, secretaria: row.get(3)?, solicitante: row.get(4)?, estado: row.get(5)?, fecha: row.get(6)? }) }).unwrap();
    let mut historial = Vec::new();
    for r in registros_iter { historial.push(r.unwrap()); }
    (StatusCode::OK, Json(historial)).into_response()
}

async fn aprobar_solicitud_api(Json(payload): Json<AprobarTicket>) -> impl IntoResponse {
    let mut conn = Connection::open(&obtener_ruta_bd()).unwrap();
    if !validar_pin_seguro(&conn, payload.tesorero_id, &payload.pin) { return (StatusCode::UNAUTHORIZED, "Denegado.".to_string()); }

    let ticket: Result<(f64, String, i32, i32), _> = conn.query_row("SELECT monto, motivo, secretaria_id, solicitante_id FROM solicitudes_egresos WHERE id = ?1 AND estado = 'PENDIENTE'", params![payload.ticket_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)));
    let (monto, motivo, sec_id, solicitante_id) = match ticket { Ok(t) => t, Err(_) => return (StatusCode::BAD_REQUEST, "Inválido.".to_string()) };

    let permiso: Result<bool, _> = conn.query_row("SELECT 1 FROM tesoreros t LEFT JOIN tesoreros_secretarias ts ON t.id = ts.tesorero_id AND ts.secretaria_id = ?2 WHERE t.id = ?1 AND (t.es_pastor = 1 OR ts.puede_autorizar = 1)", params![payload.tesorero_id, sec_id], |row| row.get(0));
    if permiso.is_err() { return (StatusCode::UNAUTHORIZED, "Sin permiso.".to_string()); }

    let tx = conn.transaction().unwrap();
    tx.execute("UPDATE solicitudes_egresos SET estado = 'APROBADO', autorizador_id = ?1, fecha_autorizacion = CURRENT_TIMESTAMP WHERE id = ?2", params![payload.tesorero_id, payload.ticket_id]).unwrap();
    tx.execute("INSERT INTO ingresos_ofrendas (monto, motivo, secretaria_id, tesorero_id) VALUES (?1, ?2, ?3, ?4)", params![-monto, motivo, sec_id, solicitante_id]).unwrap();
    tx.commit().unwrap();

    (StatusCode::OK, "Aprobado.".to_string())
}

async fn rechazar_solicitud_api(Json(payload): Json<AprobarTicket>) -> impl IntoResponse {
    let conn = Connection::open(&obtener_ruta_bd()).unwrap();
    if !validar_pin_seguro(&conn, payload.tesorero_id, &payload.pin) { return (StatusCode::UNAUTHORIZED, "Denegado.".to_string()); }

    let ticket_sec: Result<i32, _> = conn.query_row("SELECT secretaria_id FROM solicitudes_egresos WHERE id = ?1", params![payload.ticket_id], |row| row.get(0));
    let sec_id = match ticket_sec { Ok(id) => id, Err(_) => return (StatusCode::NOT_FOUND, "No encontrado".to_string()) };

    let permiso: Result<bool, _> = conn.query_row("SELECT 1 FROM tesoreros t LEFT JOIN tesoreros_secretarias ts ON t.id = ts.tesorero_id AND ts.secretaria_id = ?2 WHERE t.id = ?1 AND (t.es_pastor = 1 OR ts.puede_autorizar = 1)", params![payload.tesorero_id, sec_id], |row| row.get(0));
    if permiso.is_err() { return (StatusCode::UNAUTHORIZED, "Sin permiso.".to_string()); }

    match conn.execute("UPDATE solicitudes_egresos SET estado = 'RECHAZADO', autorizador_id = ?1, fecha_autorizacion = CURRENT_TIMESTAMP WHERE id = ?2 AND estado = 'PENDIENTE'", params![payload.tesorero_id, payload.ticket_id]) {
        Ok(_) => (StatusCode::OK, "Rechazado.".to_string()), _ => (StatusCode::BAD_REQUEST, "Ya procesado.".to_string())
    }
}