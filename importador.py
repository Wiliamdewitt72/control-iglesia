import sqlite3
import csv
import os

def importar_historico():
    print("⏳ Conectando a la base de datos...")
    
    # EL CAMBIO CLAVE: Le decimos a Python que busque la BD un nivel arriba, igual que Rust
    ruta_bd = '../iglesia.db'
    
    if not os.path.exists(ruta_bd):
        print(f"❌ ERROR: No se encontró la base de datos en '{ruta_bd}'.")
        return

    conn = sqlite3.connect(ruta_bd)
    cursor = conn.cursor()

    # Le pedimos a SQLite que nos dé su "diccionario" de nombres a IDs
    cursor.execute("SELECT nombre_secretaria, id FROM secretarias")
    mapa_secretarias = dict(cursor.fetchall())

    cursor.execute("SELECT nombre, id FROM tesoreros")
    mapa_tesoreros = dict(cursor.fetchall())

    # Validamos que la base de datos no esté vacía
    if not mapa_secretarias or not mapa_tesoreros:
        print("❌ ERROR: La base de datos está vacía. Enciende tu servidor de Rust ('cargo run') al menos una vez para que cree a los usuarios.")
        conn.close()
        return

    print(f"✅ BD conectada. (Encontramos {len(mapa_secretarias)} secretarías y {len(mapa_tesoreros)} tesoreros)")
    print("⏳ Leyendo el archivo CSV...")
    
    registros_importados = 0

    try:
        with open('plantilla_historico_iglesia.csv', 'r', encoding='utf-8-sig') as f:
            lector = csv.DictReader(f)
            
            for fila in lector:
                sec_id = mapa_secretarias.get(fila['Secretaría'])
                tes_id = mapa_tesoreros.get(fila['Responsable'])
                
                # Si el Excel trae un nombre que no existe en tu BD, nos avisa y lo salta
                if not sec_id or not tes_id:
                    print(f"⚠️ Saltando fila irreconocible: Secretaría '{fila['Secretaría']}' o Responsable '{fila['Responsable']}' no existen.")
                    continue

                # Formateamos los montos
                monto = float(fila['Monto'])
                if fila['Operación'] == 'EGRESO':
                    monto = -abs(monto)
                
                # Inyectamos directo al historial
                cursor.execute('''
                    INSERT INTO ingresos_ofrendas (monto, motivo, secretaria_id, tesorero_id, fecha_creacion)
                    VALUES (?, ?, ?, ?, ?)
                ''', (monto, fila['Motivo'], sec_id, tes_id, fila['Fecha']))
                registros_importados += 1

        # Guardamos los cambios
        conn.commit()
        print(f"🎉 ¡Éxito total! Se han inyectado {registros_importados} registros al historial.")
        
    except FileNotFoundError:
        print("❌ ERROR: No se encontró el archivo 'plantilla_historico_iglesia.csv' en esta carpeta.")
    finally:
        conn.close()

if __name__ == '__main__':
    importar_historico()
