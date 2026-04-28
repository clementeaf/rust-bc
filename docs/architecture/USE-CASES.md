# Casos de Uso y Escenarios de Aplicabilidad

Esta blockchain permisionada en Rust esta disenada para escenarios donde **multiples organizaciones que no confian entre si** necesitan compartir datos de forma segura, auditable e inmutable, sin depender de un intermediario central.

---

## Cuando usar esta blockchain

**Regla simple:** si tu caso cumple estas 3 condiciones, esta blockchain aplica:

1. **Multiples organizaciones** que necesitan compartir una verdad comun
2. **Datos sensibles o regulados** que requieren control de acceso
3. **Necesidad de auditoria** inmutable sobre quien hizo que y cuando

**Cuando NO usarla:**
- Si una sola organizacion controla todos los datos -> usa una base de datos tradicional
- Si los datos son publicos y no sensibles -> considera una blockchain publica
- Si no necesitas auditoria ni trazabilidad -> es sobreingenieria

---

## 1. Salud: Historial Medico Transfronterizo

### Problema
Un paciente chileno viaja al extranjero y sufre una emergencia medica. El personal medico local no tiene acceso a su historial, alergias, ni procedimientos previos. Hoy esto depende de llamadas telefonicas, faxes o la memoria del paciente.

### Actores (cada uno un nodo/organizacion)
- Hospitales y clinicas en Chile
- Ministerio de Salud (MINSAL)
- Redes medicas internacionales adheridas

### Como funciona

1. **Registro del procedimiento** - El hospital registra la cirugia/examen como una transaccion en la blockchain. El dato queda inmutable, con timestamp y firma digital del medico (DID).

2. **Identidad del paciente** - El paciente tiene un DID (identidad descentralizada). El es dueno de sus credenciales verificables: grupo sanguineo, alergias, cirugias previas, medicamentos activos.

3. **Acceso en el extranjero** - El paciente llega a urgencias en Madrid. El medico espanol escanea el QR del paciente, que presenta su credencial verificable. El hospital:
   - Verifica la firma criptografica contra la blockchain (autenticidad garantizada)
   - Accede al historial autorizado por el paciente
   - No necesita llamar a Chile ni a ningun intermediario

4. **Control del paciente** - El paciente decide que comparte. Puede mostrar alergias y cirugias pero ocultar salud mental. Las credenciales son selectivas.

5. **Audit trail** - Queda registro inmutable de quien accedio, cuando y que vio. Cumple regulaciones tipo GDPR o Ley 19.628 chilena de datos personales.

### Componentes utilizados

| Componente | Rol |
|---|---|
| DID + Credenciales verificables | Identidad del paciente y del medico emisor |
| Multi-org + endorsement | Cada hospital/ministerio valida las transacciones |
| Private data collections | Datos medicos solo visibles para autorizados |
| Audit trail | Registro inmutable de accesos |
| TLS mutuo | Comunicacion segura entre nodos internacionales |

### Que se construye encima
- App movil del paciente (wallet de credenciales)
- Integracion con estandares medicos (HL7 FHIR)
- Acuerdos bilaterales con redes medicas extranjeras

---

## 2. Agroexportacion: Trazabilidad de Cadena de Suministro

### Problema
Chile exporta fruta, salmon, vino y otros productos a mercados exigentes (UE, EE.UU., Asia). Los importadores exigen trazabilidad completa: origen, transporte, cadena de frio, certificaciones fitosanitarias. Hoy esto se maneja con papeles, PDFs y confianza.

### Actores
- Productores agricolas / pesqueros
- Empresas de transporte y logistica
- SAG (Servicio Agricola y Ganadero)
- Aduana
- Importadores en destino

### Como funciona

1. **Cosecha** - El productor registra: lote, fecha, ubicacion, certificacion organica. Firmado con su DID.
2. **Transporte** - La empresa logistica registra: temperatura, humedad, tiempos de transito. Cada punto de control es una transaccion.
3. **Certificacion** - SAG emite una credencial verificable de que el lote cumple normas fitosanitarias.
4. **Aduana** - Registro de salida del pais con referencia al lote trazado.
5. **Importador** - Escanea el codigo del producto y ve toda la cadena: desde la parcela hasta su bodega. Cada paso verificable criptograficamente.

### Valor diferencial
- **Automatizacion de compliance:** las certificaciones son credenciales verificables, no PDFs falsificables
- **Disputa rapida:** si un lote llega danado, el audit trail muestra exactamente donde se rompio la cadena de frio
- **Premium de mercado:** productos con trazabilidad blockchain pueden acceder a mercados premium

---

## 3. Sector Publico: Documentos y Certificados Verificables

### Problema
Titulos universitarios, certificados de antecedentes, permisos municipales, licencias profesionales... todos requieren verificacion manual (ir a una oficina, llamar, pedir una copia). Es lento, costoso y susceptible a falsificacion.

### Actores
- Universidades
- Registro Civil
- Municipalidades
- Ministerios
- Colegios profesionales
- Empleadores / entidades que verifican

### Como funciona

1. **Emision** - La universidad emite el titulo como credencial verificable firmada con su DID institucional. Queda registrado en la blockchain.
2. **Portabilidad** - El egresado guarda la credencial en su wallet digital.
3. **Verificacion** - Un empleador escanea la credencial. En segundos verifica: emisor legitimo, no revocada, datos intactos. Sin llamar a la universidad.
4. **Revocacion** - Si se descubre fraude academico, la universidad revoca la credencial. La revocacion se propaga automaticamente.

### Aplicaciones concretas
- **Titulos universitarios** - Eliminacion de titulo falso como problema
- **Certificado de antecedentes** - Verificacion instantanea sin ir al Registro Civil
- **Permisos de circulacion** - Municipalidad emite, Carabineros verifica en terreno
- **Licencias profesionales** - Colegio Medico emite, paciente verifica que su doctor esta habilitado

---

## 4. Notarial y Legal: Registros Inmutables

### Problema
El sistema notarial chileno depende de registros fisicos y copias autorizadas. Los contratos, poderes y escrituras requieren intermediacion costosa para probar su autenticidad y existencia en una fecha determinada.

### Actores
- Notarias
- Conservador de Bienes Raices
- Abogados
- Poder Judicial
- Partes contratantes

### Como funciona

1. **Protocolizacion** - El notario registra el hash del documento en la blockchain. El documento completo se almacena off-chain (private data), pero su existencia y marca temporal son inmutables.
2. **Firma multiple** - Las partes firman con sus DIDs. El endorsement policy requiere N-de-M firmas para validar.
3. **Verificacion posterior** - Ante un tribunal, se presenta el documento y se verifica que el hash coincide con el registrado en la blockchain en la fecha indicada.
4. **Cadena de custodia** - Cada transferencia de propiedad, cesion de derechos o modificacion queda trazada.

### Valor diferencial
- **Prueba de existencia** temporal irrefutable (no depende de que la notaria conserve sus libros)
- **Reduccion de fraude** notarial (los registros no se pueden alterar retroactivamente)
- **Agilidad** en verificacion de documentos entre instituciones

---

## 5. Finanzas: Conciliacion Interinstitucional

### Problema
Bancos, aseguradoras, AFP, cajas de compensacion y otras instituciones financieras intercambian informacion constantemente (pagos, cobros, conciliaciones). Cada una mantiene su propia version de la verdad, y las discrepancias generan costos operativos enormes.

### Actores
- Bancos
- Aseguradoras
- AFP / fondos de pensiones
- Camara de Compensacion
- CMF (Comision para el Mercado Financiero)

### Como funciona

1. **Registro compartido** - Cada transaccion interinstitucional se registra en la blockchain. Ambas partes firman (endorsement de 2 orgs).
2. **Conciliacion automatica** - No hay "mi version" vs "tu version". La blockchain es la unica fuente de verdad compartida.
3. **Regulador como observador** - La CMF opera un nodo con permisos de lectura. Puede auditar en tiempo real sin solicitar reportes.
4. **Liquidacion** - Las operaciones liquidadas quedan marcadas como finales. El audit trail completo esta disponible para auditoria.

### Valor diferencial
- **Eliminacion de conciliacion manual** (hoy puede tomar dias)
- **Reduccion de disputas** al tener una sola verdad compartida
- **Compliance en tiempo real** para el regulador

---

## 6. Educacion: Certificaciones y Microcredenciales

### Problema
El mercado laboral evoluciona mas rapido que los titulos universitarios. Cursos cortos, bootcamps, certificaciones tecnicas y microcredenciales proliferan, pero no tienen un sistema estandarizado de verificacion.

### Actores
- Instituciones educativas (universidades, CFT, IP)
- Plataformas de educacion online (Coursera, Platzi, etc.)
- SENCE
- Empleadores

### Como funciona

1. **Emision estandarizada** - Cada institucion emite credenciales verificables con un formato comun: que se aprendio, cuantas horas, que competencias certifica.
2. **Portfolio acumulativo** - El profesional acumula credenciales de multiples fuentes en su wallet. Su perfil profesional es verificable, no un PDF editable.
3. **Matching laboral** - Un empleador busca "Python + AWS + Data Science". El sistema verifica automaticamente que las credenciales del candidato son autenticas.
4. **SENCE como validador** - SENCE puede endorsar que un curso cumple estandares de calidad. Esa certificacion es tambien una credencial verificable.

---

## 7. Supply Chain Farmaceutico

### Problema
Medicamentos falsificados son un problema global. La cadena farmaceutica (laboratorio -> distribuidor -> farmacia -> paciente) necesita trazabilidad completa para garantizar autenticidad y condiciones de almacenamiento.

### Actores
- Laboratorios farmaceuticos
- Distribuidoras
- Farmacias
- ISP (Instituto de Salud Publica)
- Pacientes

### Como funciona

1. **Fabricacion** - El laboratorio registra cada lote: principio activo, fecha de fabricacion, vencimiento, condiciones de almacenamiento.
2. **Distribucion** - Cada punto de la cadena registra recepcion y despacho. Temperatura y humedad en transito.
3. **Farmacia** - Al recibir, verifica autenticidad del lote contra la blockchain. Si no coincide, rechaza.
4. **Paciente** - Escanea el codigo del medicamento y verifica que es autentico, no vencido, y fue almacenado correctamente.
5. **ISP como regulador** - Nodo observador con capacidad de alerta automatica ante anomalias.

### Valor diferencial
- **Eliminacion de medicamentos falsificados** en la cadena formal
- **Recall eficiente** - Si un lote tiene problemas, se identifica instantaneamente donde esta cada unidad
- **Cumplimiento regulatorio** automatizado

---

## Arquitectura Comun

Todos los casos de uso comparten la misma arquitectura base:

```
+------------------+     +------------------+     +------------------+
|   Org A (Nodo)   |<--->|   Org B (Nodo)   |<--->|   Org C (Nodo)   |
|   Hospital       |     |   MINSAL         |     |   Red Extranjera |
|   TLS mutuo      |     |   TLS mutuo      |     |   TLS mutuo      |
+------------------+     +------------------+     +------------------+
        |                         |                         |
        v                         v                         v
+------------------------------------------------------------------+
|                    Blockchain Compartida                          |
|  - Transacciones firmadas (DID)                                  |
|  - Credenciales verificables                                     |
|  - Endorsement policies por organizacion                         |
|  - Private data collections (datos sensibles)                    |
|  - Audit trail inmutable                                         |
|  - Consenso Raft                                                 |
+------------------------------------------------------------------+
        |
        v
+------------------------------------------------------------------+
|                    Aplicaciones (sobre la blockchain)             |
|  - Apps moviles / web                                            |
|  - Integraciones con sistemas existentes                         |
|  - Dashboards de monitoreo                                       |
+------------------------------------------------------------------+
```

### Componentes reutilizados en cada caso

| Componente | Funcion |
|---|---|
| **DID + Credenciales** | Identidad de personas e instituciones |
| **Multi-org + Endorsement** | Validacion distribuida entre organizaciones |
| **Private Data Collections** | Datos sensibles compartidos solo entre partes autorizadas |
| **Audit Trail** | Registro inmutable de todas las operaciones |
| **TLS Mutuo** | Comunicacion segura entre nodos |
| **Consenso Raft** | Acuerdo sobre el orden de transacciones |
| **Chaincode** | Logica de negocio ejecutada en la blockchain |
| **API REST** | Integracion con aplicaciones externas |

---

## Ventajas frente a alternativas

| Criterio | Esta Blockchain | Hyperledger Fabric | Blockchain Publica | Base de Datos Central |
|---|---|---|---|---|
| Multi-organizacion | Si | Si | Si | No (requiere confianza central) |
| Datos privados | Si (collections) | Si | No (todo publico) | Si |
| Auditoria inmutable | Si | Si | Si | Depende de implementacion |
| Complejidad operativa | Baja (1 binario) | Alta (Docker, CAs, Orderers) | Media | Baja |
| Costo por transaccion | Ninguno | Ninguno | Gas fees | Ninguno |
| Rendimiento | Alto (Rust nativo) | Medio (Go + Docker) | Bajo (consenso global) | Alto |
| Soberania de datos | Total | Total | Ninguna | Total |
| Lenguaje | Rust | Go/Java | Solidity | Varios |
