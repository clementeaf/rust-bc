# Cerulean Ledger — Capacidades Horizontales

Documento de referencia para reportes. Cada capacidad es independiente, reutilizable por cualquier vertical, y está en producción.

---

## 1. Inmutabilidad

**Garantía:** Ningún registro puede ser alterado después de ser escrito.

- Cada bloque referencia el hash del anterior — alterar uno invalida toda la cadena
- Firmas digitales por transacción vinculadas a identidad del autor
- Merkle root por bloque permite detectar cualquier modificación

**Métrica:** 0 alteraciones posibles sin detección (verificable matemáticamente)
**Aplica a:** Credenciales, voto, supply chain, auditoría

---

## 2. Criptografía post-cuántica

**Garantía:** Los registros de hoy seguirán protegidos cuando existan computadores cuánticos.

- ML-DSA-65 (FIPS 204, 2024) — firmas digitales
- SHA3-256 (FIPS 202) — hashing
- ML-KEM-768 (FIPS 203) — intercambio de claves
- Dual-signing — firma clásica + PQC simultánea para migración gradual
- Módulo criptográfico con self-tests automáticos y zeroización de claves

**Métrica:** Estándares NIST publicados agosto 2024. Mismos que adoptan Defensa USA y UE.
**Aplica a:** Credenciales (títulos válidos 30+ años), voto (verificable a futuro), finanzas (regulador lo exigirá)

---

## 3. Canales (aislamiento de datos)

**Garantía:** Cada organización o consorcio opera en un espacio completamente separado.

- Ledger independiente por canal (bloques, transacciones, world state)
- Una organización no puede leer datos de otro canal
- Canales creados en runtime sin reiniciar la red

**Métrica:** Aislamiento total — no existe API que cruce datos entre canales
**Aplica a:** Supply chain (un consorcio por canal), finanzas (un banco ve solo lo suyo), voto (una elección por canal)

---

## 4. Control de acceso (ACL + MSP)

**Garantía:** Nadie accede a nada a menos que esté explícitamente autorizado.

- Deny-by-default — toda operación está prohibida por defecto
- 3 roles: Admin, Peer, Client — extraídos del certificado X.509
- Autenticación via mTLS (certificados, no contraseñas)
- ACL por recurso — cada endpoint tiene su política

**Métrica:** 0 accesos sin autorización explícita
**Aplica a:** Todas las verticales

---

## 5. Identidad descentralizada (DID)

**Garantía:** Cada participante controla su propia identidad. Nadie más puede revocarla o falsificarla.

- Formato `did:cerulean:identificador`
- Credenciales verificables emitidas entre DIDs
- Verificación criptográfica en milisegundos
- PIN (Argon2id) para autenticación simple de usuarios finales

**Métrica:** Verificación de credencial en <50ms vs 3-15 días hábiles manual
**Aplica a:** Credenciales (emisión/verificación), voto (identidad del votante), supply chain (identidad de participantes)

---

## 6. Gobernanza on-chain

**Garantía:** Las decisiones colectivas quedan registradas, votadas y auditables. No hay decisiones unilaterales opacas.

- Propuestas con ciclo completo: submit → vote → pass → timelock → execute
- Votación ponderada con quorum y umbral configurable
- Cambios de parámetros del protocolo via gobernanza
- Auditoría pública de resultados sin revelar votos individuales

**Métrica:** 100% de decisiones de gobernanza auditables por cualquier observador
**Aplica a:** Voto (es el producto mismo), supply chain (cambios de reglas), finanzas (cambios de política)

---

## 7. Smart contracts (Wasm + EVM)

**Garantía:** Reglas de negocio que se ejecutan automáticamente, de forma verificable y sin intermediarios.

- Wasmtime: fuel metering + memory limits, sin Docker
- revm: compatible con Solidity, MetaMask, Hardhat
- Lifecycle: install → approve → commit (multi-org)
- Upgrade manager con aprobación multi-organización

**Métrica:** Dos runtimes disponibles sin cambiar infraestructura
**Aplica a:** Finanzas (lógica programable), supply chain (reglas automatizadas)

---

## 8. Ejecución paralela

**Garantía:** Miles de operaciones simultáneas sin cuellos de botella artificiales.

- Wave scheduler agrupa transacciones sin conflictos para ejecución concurrente
- Detección de conflictos RAW/WAW/WAR
- MVCC valida lecturas/escrituras al commit
- ~18,700 TX/s (motor), ~42 TPS (end-to-end HTTP con rate limiting)

**Métrica:** 4-9x más rápido que Hyperledger Fabric en micro-benchmarks
**Aplica a:** Credenciales (emisión masiva), voto (miles de votos simultáneos), supply chain (checkpoints)

---

## 9. Consenso multi-modal

**Garantía:** La red acuerda la verdad sin depender de una autoridad central. Modelo seleccionable según el caso.

| Modo | Tolerancia | Cuándo usarlo |
|---|---|---|
| Raft | Nodos caídos | Consorcio confiable, máxima velocidad |
| BFT (HotStuff) | Nodos maliciosos | Red con adversarios potenciales |
| DAG | Alta concurrencia | Múltiples propuestas simultáneas |
| DPoS | Selección por stake | Redes abiertas con validadores |

**Métrica:** Finalidad determinística — un bloque committeado es final, sin esperas
**Aplica a:** Todas las verticales

---

## 10. Verificación independiente (Light client)

**Garantía:** Un tercero puede verificar cualquier dato sin acceder al sistema completo.

- Cadena de headers compacta con verificación de QC (quorum certificate)
- Merkle proofs contra headers sincronizados
- Funciona desde dispositivos móviles e IoT

**Métrica:** Verificación desde celular sin instalar nodo completo
**Aplica a:** Credenciales (verificar título desde celular), voto (auditar resultado)

---

## 11. Observabilidad y notificación

**Garantía:** Todo lo que ocurre en la red es monitoreado y notificable en tiempo real.

- WebSocket: eventos de bloques, transacciones y chaincode en tiempo real
- CSIRT webhook: eventos de seguridad reenviados automáticamente a SIEM/CSIRT
- Prometheus + Grafana: dashboards de salud, rendimiento y seguridad
- Health endpoint con status de todos los subsistemas

**Métrica:** Detección de incidentes en tiempo real, integrable con ANCI (Ley 21.663)
**Aplica a:** Todas las verticales

---

## 12. Soberanía operacional

**Garantía:** La plataforma corre en tu infraestructura. No dependes de terceros, clouds, ni tokens públicos.

- Self-hosted (Docker, bare metal, o cloud privada)
- Sin dependencia de blockchain pública ni token cotizado
- Sandbox desplegable con un solo comando
- SDKs en JavaScript y Python para integración

**Métrica:** 0 dependencias externas para operar
**Aplica a:** Todas las verticales — requisito regulatorio en gobierno y banca

---

## Resumen cuantitativo

| Indicador | Valor |
|---|---|
| Capacidades horizontales | 12 |
| Componentes en producción | 57 de 58 |
| Tests automatizados | 1,445 |
| Throughput motor | ~18,700 TX/s |
| Throughput E2E (HTTP) | ~42 TPS |
| Latencia p50 | 14ms |
| Estándares criptográficos | FIPS 204, 202, 203 (NIST 2024) |
| Compliance | Ley 21.663 (Chile) — mapeo completo |
| Verticales habilitadas | 4 (credenciales, voto, supply chain, finanzas) |
