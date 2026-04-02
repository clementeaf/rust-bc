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
