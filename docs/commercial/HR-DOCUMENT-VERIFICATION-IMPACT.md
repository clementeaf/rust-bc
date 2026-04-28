# Impacto de rust-bc en la Verificacion Documental para Contrataciones de Personal

## Problema actual

La verificacion de documentos en procesos de contratacion presenta fricciones significativas:

- **Proceso manual y lento**: verificar titulos, certificaciones y antecedentes laborales requiere contactar instituciones una por una, con tiempos de respuesta de dias a semanas.
- **Fraude documental**: no existe un mecanismo estandar para validar la autenticidad de un documento sin depender de la institucion emisora.
- **Duplicacion de esfuerzo**: cada empresa repite el mismo proceso de verificacion para el mismo candidato, sin posibilidad de reutilizar verificaciones previas.
- **Sin revocacion en tiempo real**: si una certificacion expira o un titulo es revocado, no hay notificacion automatica a las partes que lo verificaron previamente.
- **Datos sensibles dispersos**: informacion personal del candidato queda almacenada en multiples sistemas sin control del titular.

---

## Solucion: Credenciales Verificables sobre Blockchain

rust-bc implementa un stack completo de identidad descentralizada y credenciales verificables que resuelve cada uno de estos problemas.

### Capacidades de la red aplicadas a RRHH

| Capacidad | Aplicacion en contrataciones |
|---|---|
| **DID (Identidad Descentralizada)** | Cada candidato, universidad e institucion tiene un identificador unico y autosoberano, sin depender de un tercero centralizado. |
| **Verifiable Credentials** | Titulos universitarios, certificaciones profesionales y antecedentes laborales se emiten como credenciales digitales firmadas con campos `issuer_did`, `subject_did`, `cred_type`, `expires_at` y `revoked_at`. |
| **Firma Post-Cuantica (ML-DSA-65)** | Credenciales firmadas con algoritmo resistente a computacion cuantica (FIPS 204), preparado para regulacion futura y proteccion a largo plazo. |
| **Revocacion on-chain** | Si una universidad revoca un titulo o una certificacion expira, el campo `revoked_at` se actualiza y cualquier verificacion posterior lo refleja inmediatamente. |
| **Channels (aislamiento Fabric-style)** | Cada empresa o consorcio de RRHH opera en su propio canal privado con ledger y world state independientes. Los datos de contratacion de una empresa no son visibles para otra. |
| **Private Data Collections** | Datos personales sensibles (antecedentes penales, informacion medica) se almacenan en colecciones privadas, visibles unicamente para las partes autorizadas. |
| **Audit trail inmutable** | Cada emision, verificacion y revocacion queda registrada en la blockchain. Quien verifico que documento, cuando, y con que resultado: cumplimiento regulatorio automatico. |
| **ACL + mTLS** | Solo entidades autorizadas (universidades registradas, empresas verificadoras) pueden emitir o consultar credenciales. Autenticacion mutua por certificado TLS. |

---

## Flujo de verificacion documental

```
Paso 1: Emision
  Universidad / Institucion certificadora (issuer)
    → Emite credencial digital al DID del candidato
    → Firma con clave post-cuantica (ML-DSA-65 o Ed25519)
    → Queda registrada on-chain con timestamp inmutable

Paso 2: Presentacion
  Candidato (subject)
    → Comparte su DID con la empresa contratante
    → La empresa consulta todas las credenciales asociadas a ese DID

Paso 3: Verificacion
  Empresa contratante (verifier)
    → Verifica on-chain: firma valida, no revocada, no expirada
    → Consulta el DID del emisor para confirmar que es una institucion reconocida
    → Verificacion instantanea, sin contactar a la institucion

Paso 4: Registro de auditoria
  La verificacion queda registrada como evento inmutable
    → Cumplimiento de normativas de proteccion de datos
    → Trazabilidad completa del proceso de contratacion
```

---

## Endpoints disponibles (API REST)

La red expone endpoints listos para integracion con sistemas de RRHH:

| Operacion | Endpoint | Metodo |
|---|---|---|
| Registrar identidad (DID) | `/api/v1/store/identities` | POST |
| Consultar identidad | `/api/v1/store/identities/{did}` | GET |
| Emitir credencial | `/api/v1/store/credentials` | POST |
| Verificar credencial | `/api/v1/store/credentials/{id}` | GET |
| Credenciales de un candidato | `/api/v1/store/credentials/by-subject/{did}` | GET |
| Crear canal privado | `/api/v1/channels/create` | POST |
| Listar canales | `/api/v1/channels/list` | GET |

### Ejemplo: emitir un titulo universitario

```json
POST /api/v1/store/credentials
{
  "id": "cred-titulo-ingenieria-2024-001",
  "issuer_did": "did:rustbc:universidad-nacional",
  "subject_did": "did:rustbc:candidato-juan-perez",
  "cred_type": "titulo_universitario",
  "issued_at": 1713398400,
  "expires_at": 0,
  "revoked_at": null
}
```

### Ejemplo: verificar credenciales de un candidato

```json
GET /api/v1/store/credentials/by-subject/did:rustbc:candidato-juan-perez

Response:
[
  {
    "id": "cred-titulo-ingenieria-2024-001",
    "issuer_did": "did:rustbc:universidad-nacional",
    "subject_did": "did:rustbc:candidato-juan-perez",
    "cred_type": "titulo_universitario",
    "issued_at": 1713398400,
    "expires_at": 0,
    "revoked_at": null
  },
  {
    "id": "cred-cert-pmp-2024-045",
    "issuer_did": "did:rustbc:pmi-global",
    "subject_did": "did:rustbc:candidato-juan-perez",
    "cred_type": "certificacion_profesional",
    "issued_at": 1710720000,
    "expires_at": 1742256000,
    "revoked_at": null
  }
]
```

---

## Ventaja competitiva frente a soluciones existentes

| Aspecto | Verificacion manual | BD centralizada | rust-bc |
|---|---|---|---|
| Tiempo de verificacion | Dias a semanas | Minutos (si hay acceso) | Segundos |
| Resistencia a fraude | Baja | Media | Alta (firma criptografica) |
| Revocacion en tiempo real | No | Depende del proveedor | Si (on-chain) |
| Privacidad del candidato | Baja (datos en multiples sistemas) | Baja (vendor controla datos) | Alta (channels + private data) |
| Vendor lock-in | N/A | Alto | Ninguno (open source) |
| Punto unico de fallo | Cada institucion | Proveedor central | Ninguno (distribuido) |
| Preparacion post-cuantica | No | No | Si (ML-DSA-65, FIPS 204) |
| Auditoria regulatoria | Manual | Parcial | Automatica e inmutable |

---

## Tipos de documentos verificables

La red soporta cualquier tipo de credencial. Ejemplos relevantes para contrataciones:

- Titulos universitarios (grado, maestria, doctorado)
- Certificaciones profesionales (PMP, AWS, Cisco, etc.)
- Antecedentes laborales (empleador anterior confirma periodo y cargo)
- Antecedentes penales (emitidos por autoridad competente)
- Certificados medicos de aptitud laboral
- Licencias profesionales (medica, legal, ingenieria)
- Cursos y formacion continua
- Referencias laborales verificadas

---

## Arquitectura de privacidad para RRHH

```
                    Canal "empresa-abc"
                    (ledger aislado)
                   ┌─────────────────┐
                   │  Credenciales   │
                   │  verificadas    │
                   │  para empresa   │
                   │  ABC solamente  │
                   └─────────────────┘

  Canal "consorcio-rrhh"           Private Data Collection
  (multiples empresas)             "antecedentes-penales"
 ┌─────────────────────┐         ┌─────────────────────┐
 │  Pool compartido    │         │  Solo visible para   │
 │  de credenciales    │         │  empresa contratante │
 │  pre-verificadas    │         │  y candidato         │
 └─────────────────────┘         └─────────────────────┘
```

- **Channels**: cada empresa o consorcio opera en aislamiento total
- **Private Data Collections**: datos sensibles con acceso granular
- **ACL por organizacion**: solo entidades con identidad TLS verificada pueden operar

---

## Interfaz visual

El Block Explorer incluye paginas dedicadas para:

- **Identity**: registro y consulta de DIDs
- **Credentials**: emision, consulta y verificacion de credenciales
- **Channels**: gestion de canales privados

Accesible en `http://localhost:5173` con el nodo corriendo.

---

## Siguiente paso: integracion

Para integrar con un sistema de RRHH existente:

1. **Registrar la empresa como DID** en la red
2. **Registrar instituciones emisoras** (universidades, certificadoras) como DIDs
3. **Las instituciones emiten credenciales** al DID de cada candidato
4. **El sistema de RRHH consulta la API** para verificar credenciales en tiempo real
5. **Configurar un canal privado** si se requiere aislamiento de datos entre empresas

La integracion es via API REST estandar — compatible con cualquier lenguaje o plataforma.
