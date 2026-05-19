import csv
import random
from datetime import datetime, timedelta

def generar_datos_prueba():
    secretarias = ['Niños', 'Educacional', 'General', 'Varones']
    
    # Mapeo de responsables legítimos según tu base de datos
    responsables_por_sec = {
        'Niños': 'Susana Frias',
        'Educacional': 'Susana Frias',
        'General': 'Juan Rios',
        'Varones': 'Juan Rios'
    }
    
    # Catálogo de motivos realistas para que la base de datos se vea natural
    motivos_ingreso = {
        'Niños': ['Ofrenda escuela dominical', 'Donación material didáctico', 'Diezmo niños', 'Recaudación kermés infantil'],
        'Educacional': ['Ofrenda apoyo instituto', 'Venta de libros eclesiales', 'Inscripción taller familias', 'Donación biblioteca'],
        'General': ['Ofrenda general domingo', 'Diezmos mensuales congregación', 'Ofrenda de primicias', 'Donación mantenimiento'],
        'Varones': ['Ofrenda fraternidad varones', 'Recaudación desayuno varones', 'Donación proyecto construcción', 'Cuotas retiro']
    }

    motivos_egreso = {
        'Niños': ['Compra de dulces y galletas', 'Material de papelería', 'Decoración salón infantil', 'Regalos día del niño'],
        'Educacional': ['Pago copias e impresiones', 'Honorarios profesor invitado', 'Compra de proyectores', 'Libros de texto'],
        'General': ['Pago de luz del templo', 'Reparación sistema sonido', 'Artículos de limpieza', 'Recibo de internet oficina'],
        'Varones': ['Alimentos para desayuno', 'Herramientas mantenimiento', 'Gasolina transporte de jóvenes', 'Folletos evangelismo']
    }

    # Fecha de inicio: 1 de Enero de 2026
    fecha_base = datetime(2026, 1, 1, 10, 0, 0)

    print("⏳ Generando 200 registros balanceados...")

    with open('plantilla_historico_iglesia.csv', 'w', newline='', encoding='utf-8-sig') as f:
        lector = csv.writer(f)
        # Encabezados exactos que espera tu script importador
        lector.writerow(['Fecha', 'Operación', 'Monto', 'Secretaría', 'Motivo', 'Responsable'])

        for i in range(200):
            # Distribución equitativa: 200 / 4 = exactamente 50 registros por secretaría
            sec = secretarias[i % len(secretarias)]
            
            # Esparcimos los días a lo largo de 135 días (Ene a Mayo 2026) para armar una curva bonita
            dias_extra = random.randint(0, 135)
            horas_extra = random.randint(0, 12)
            min_extra = random.randint(0, 59)
            fecha_registro = fecha_base + timedelta(days=dias_extra, hours=horas_extra, minutes=min_extra)
            fecha_str = fecha_registro.strftime('%Y-%m-%d %H:%M:%S')

            # 70% Ingresos y 30% Egresos para mantener las finanzas sanas y en positivo
            operacion = 'INGRESO' if random.random() < 0.72 else 'EGRESO'

            if operacion == 'INGRESO':
                # La secretaría General maneja montos más grandes
                monto = round(random.uniform(1000, 6000) if sec == 'General' else random.uniform(150, 1500), 2)
                motivo = random.choice(motivos_ingreso[sec])
            else:
                monto = round(random.uniform(500, 3000) if sec == 'General' else random.uniform(80, 700), 2)
                motivo = random.choice(motivos_egreso[sec])

            # El 80% de las veces lo registra su encargado, el 20% el Pastor Jacobo
            responsable = responsables_por_sec[sec] if random.random() < 0.8 else 'Jacobo Espinosa'

            lector.writerow([fecha_str, operacion, monto, sec, motivo, responsable])

    print("🎉 ¡Éxito! Archivo 'plantilla_historico_iglesia.csv' creado con 200 filas.")

if __name__ == '__main__':
    generar_datos_prueba()
