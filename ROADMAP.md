# Roadmap rust-bc: evolución hacia capacidades tipo Hyperledger Fabric

La idea es **mejorar lo que ya existe** y acercarlo a lo que ofrece un stack **enterprise permissioned** como [Hyperledger Fabric](https://hyperledger-fabric.readthedocs.io/en/release-2.5/) (red conocida, identidades, políticas, operación documentada)—no sustituir el proyecto de la noche a la mañana, sino **cerrar huecos por fases**.

**Aviso legal (datos de salud / RGPD, etc.):** las secciones finales recuerdan cumplimiento; no sustituyen asesoría legal ni certificación.

---

## Mapa conceptual: Fabric ↔ rust-bc hoy ↔ dirección

| Dimensión (Fabric) | rust-bc hoy | Próximos pasos concretos |
|--------------------|-------------|---------------------------|
| **Red permisionada** (solo participantes conocidos) | P2P + API; límites parciales (API keys) | Lista blanca de nodos / certificados; política de admisión a la red |
| **Identidad criptográfica** (MSP, certificados) | Direcciones y firmas de wallet | PKI o CA interna; roles (admin, peer, cliente); rotación y revocación |
| **Consenso / ordenación** | PoW + staking en evolución | Documentar modelo objetivo; opción de BFT/Raft-like o comité de ordenación en red cerrada |
| **Ledger + estado mundial** | Cadena + estado reconstruible | Contratos de lectura/escritura explícitos; snapshots ya existentes → formalizar compatibilidad y migración |
| **Chaincode / lógica de negocio** | Smart contracts en repo | Versionado de contratos; políticas de quién puede desplegar y actualizar |
| **Datos privados / canales** (subconjuntos de participantes) | No equivalente | Diseño off-chain cifrado + hash on-chain; o “canales” lógicos en capa de aplicación |
| **Políticas de endoso** (N-de-M) | Validación en nodo | Reglas explícitas: cuántos validadores / firmas para aceptar un bloque o una transacción |
| **TLS, operación, HA** | HTTP, logs básicos | TLS entre nodos y clientes; guías de despliegue (inspiradas en [ops de Fabric](https://hyperledger-fabric.readthedocs.io/en/release-2.5/ops_guide.html)) |

---

## Fase A — Base “enterprise-lite” (prioridad alta)

1. **Modelo de red:** definir en documentación y código quién es “miembro” (nodo + claves); rechazar conexiones fuera de política. *(Criterio: [`docs/NETWORK_MEMBERSHIP.md`](docs/NETWORK_MEMBERSHIP.md); lista blanca P2P: variable `PEER_ALLOWLIST`.)*
2. **Identidad:** pasar de solo “address” a **cadena de confianza** (aunque sea con una CA mínima y certificados autofirmados gestionados).
3. **API y P2P:** **TLS** obligatorio en entornos no locales; separar credenciales de operador vs usuario de aplicación.
4. **Observabilidad:** métricas y auditoría de acceso coherentes con un despliegue multi-nodo.

## Fase B — Gobernanza de cadena y contratos

1. **Políticas:** escribir reglas (quién firma bloques, quién despliega lógica) en configuración versionada.
2. **Smart contracts:** ciclo de vida (versión, migración, desactivación) alineado con [conceptos de chaincode](https://hyperledger-fabric.readthedocs.io/en/release-2.5/key_concepts.html) aunque la implementación siga siendo propia.
3. **Integridad de datos sensibles:** contenido clínico **fuera de la cadena** por defecto; cadena con **compromisos** (hash + metadatos).

## Fase C — Paridad funcional avanzada (según necesidad)

1. **Particionamiento:** equivalente a canales o colecciones privadas—definir quién ve qué evento sin exponer datos a toda la red.
2. **Consenso:** si la red es pequeña y cerrada, evaluar consenso más predecible que PoW puro para producción.
3. **Pruebas y hardening:** tests de carga, fuzzing de protocolo, revisión de dependencias (similar en espíritu a “deploying a production network” en la [documentación de Fabric](https://hyperledger-fabric.readthedocs.io/en/release-2.5/deployment_guide_overview.html)).

---

## Columna paralela: cumplimiento (solo si el dominio es salud o datos personales)

| Fase | Enfoque |
|------|---------|
| **C0** | Marco legal (RGPD / normativa local), DPIA, roles (responsable, encargado, DPO). |
| **C1** | Minimización de datos, retención, modelo de identidad paciente/profesional. |
| **C2** | Cifrado en reposo, MFA, logs de acceso, continuidad. |
| **C3** | Versionado y rectificación sin borrar auditoría; anclajes on-chain vs almacenamiento off-chain. |
| **C4** | Incidentes, gestión de cambios, formación. |

*(Detalle en versiones anteriores del roadmap; mantener alineado con asesoría legal.)*

---

## Relación con este repositorio

- **Sí se puede** avanzar de forma incremental: identidad, TLS, políticas y modelo de datos son trabajo **acotado y medible**.
- **No se pretende** clonar Fabric: se usa como **referencia de capacidades** ([intro y conceptos clave](https://hyperledger-fabric.readthedocs.io/en/release-2.5/key_concepts.html)).
- Objetivo: que rust-bc **se acerque en seriedad operativa y modelo de red** a un ledger permisionado enterprise, manteniendo el código propio.

---

*Revisar periódicamente con prioridades de producto; la documentación oficial de Hyperledger Fabric es la referencia externa para terminología y buenas prácticas de red permisionada.*
