# Membresía de red P2P

**Regla:** En producción solo son miembros de confianza los nodos cuya dirección P2P (`IP:puerto`) está **explícitamente incluida** en la lista autorizada del despliegue; el resto no se considera parte de la red hasta que un operador la añada.

**Lista inicial de ejemplo (sustituir por la real):**

- `127.0.0.1:8081` (solo desarrollo local)
- *(añadir aquí cada peer de staging/producción)*

## Implementación

Variable de entorno **`PEER_ALLOWLIST`**: lista separada por comas de direcciones `IP:puerto` (IPv6 entre corchetes, p. ej. `[::1]:8081`). Si está **definida** y contiene al menos una dirección válida, el servidor P2P **solo acepta conexiones entrantes** cuya dirección remota coincida con una de la lista. Si no está definida o no hay entradas válidas, el comportamiento es el de siempre (sin filtro en aceptación).

Ejemplo:

```bash
export PEER_ALLOWLIST="127.0.0.1:8081,10.0.0.5:8081"
```

## Certificate Pinning TLS

El pinning de certificados añade una segunda línea de defensa sobre la validación CA habitual: aunque un certificado esté firmado por una CA de confianza, solo se aceptan los nodos cuyo certificado coincida con un fingerprint conocido.

### Variable de entorno

| Variable | Obligatoria | Descripción |
|----------|-------------|-------------|
| `TLS_PINNED_CERTS` | No | Lista de fingerprints SHA-256 (hex) separados por coma. Si está ausente o vacía, el pinning está **desactivado** y se acepta cualquier cert válido. |

### Comportamiento

- **Sin `TLS_PINNED_CERTS`** (o vacío): solo se valida la cadena CA (comportamiento TLS/mTLS habitual).
- **Con `TLS_PINNED_CERTS`**: tras la validación CA, se comprueba que el fingerprint SHA-256 del cert de extremo (`end-entity`) esté en la lista. Si no coincide, el handshake se rechaza.
- El pinning aplica tanto a conexiones salientes (cert del servidor, vía `PinningServerCertVerifier`) como a mTLS entrante (cert del cliente, vía `PinningClientCertVerifier`).
- `TLS_VERIFY_PEER=false` desactiva toda verificación, incluido el pinning — solo para desarrollo.

### Calcular un fingerprint

El fingerprint es el SHA-256 de los bytes DER del certificado (no del PEM, sino del DER subyacente):

```bash
# A partir de un archivo PEM
openssl x509 -in peer.pem -outform DER | openssl dgst -sha256 -hex | awk '{print $2}'
```

El resultado es una cadena de 64 caracteres hexadecimales, por ejemplo:

```
a3f1c2e4b5d6789012345678901234567890abcdef1234567890abcdef123456
```

### Ejemplo de despliegue

```bash
# Pinning de dos nodos conocidos (fingerprints de ejemplo)
export TLS_PINNED_CERTS="a3f1c2e4b5d6789012345678901234567890abcdef1234567890abcdef123456,\
b7e8d9f0a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7"

# Combinado con mTLS
export TLS_MUTUAL=true
export TLS_CA_CERT_PATH=/etc/bc/ca.pem
export TLS_CERT_PATH=/etc/bc/node.pem
export TLS_KEY_PATH=/etc/bc/node.key
export TLS_PINNED_CERTS="a3f1c2e4b5d6789012345678901234567890abcdef1234567890abcdef123456"
```

### Añadir un nuevo nodo

1. Obtener el PEM del cert del nuevo nodo.
2. Calcular su fingerprint con el comando anterior.
3. Añadir el fingerprint a `TLS_PINNED_CERTS` en todos los nodos existentes.
4. Reiniciar los nodos para que recojan la nueva lista.

> **Nota:** durante una rotación de certificados, incluir temporalmente tanto el fingerprint antiguo como el nuevo para evitar interrupciones del servicio.
