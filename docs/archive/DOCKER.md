# 🐳 Guía de Deployment con Docker

## 📋 Requisitos

- Docker 20.10+
- Docker Compose 2.0+ (opcional, para multi-nodo)

## 🚀 Inicio Rápido

### Opción 1: Docker Compose (Recomendado para desarrollo)

```bash
# Construir e iniciar 3 nodos
docker-compose up -d

# Ver logs
docker-compose logs -f

# Detener
docker-compose down
```

### Opción 2: Docker Run (Producción)

```bash
# Construir imagen
docker build -t rust-bc:latest .

# Ejecutar nodo
docker run -d \
  --name rust-bc-node \
  -p 8080:8080 \
  -p 8081:8081 \
  -v blockchain-data:/app/data \
  rust-bc:latest

# Ver logs
docker logs -f rust-bc-node
```

## 📦 Construcción de Imagen

### Build estándar

```bash
docker build -t rust-bc:latest .
```

### Build con cache optimizado

```bash
# Primera vez
docker build -t rust-bc:latest .

# Rebuilds (más rápido)
docker build --cache-from rust-bc:latest -t rust-bc:latest .
```

### Build multi-arch (para diferentes plataformas)

```bash
docker buildx build --platform linux/amd64,linux/arm64 -t rust-bc:latest .
```

## 🔧 Configuración

### Variables de Entorno

| Variable | Default | Descripción |
|----------|---------|-------------|
| `API_PORT` | 8080 | Puerto para API REST |
| `P2P_PORT` | 8081 | Puerto para red P2P |
| `DB_NAME` | blockchain | Nombre de la base de datos |
| `DIFFICULTY` | 1 | Dificultad de minería |
| `RUST_LOG` | info | Nivel de logging |

### Ejemplo con variables personalizadas

```bash
docker run -d \
  --name rust-bc-node \
  -p 3000:3000 \
  -p 4000:4000 \
  -e API_PORT=3000 \
  -e P2P_PORT=4000 \
  -e DIFFICULTY=2 \
  -e RUST_LOG=debug \
  -v blockchain-data:/app/data \
  rust-bc:latest
```

## 🌐 Red P2P Multi-Nodo

### Conectar nodos manualmente

```bash
# Nodo 1 (bootstrap)
docker run -d --name node1 -p 8080:8080 -p 8081:8081 rust-bc:latest

# Nodo 2 (conectar a node1)
docker run -d --name node2 -p 8082:8080 -p 8083:8081 \
  --link node1:node1 \
  rust-bc:latest

# Conectar node2 a node1 (desde dentro del contenedor)
docker exec node2 curl -X POST http://localhost:8080/api/v1/peers/127.0.0.1:8081/connect
```

### Usando Docker Compose

```bash
# Iniciar red de 3 nodos
docker-compose up -d

# Conectar nodos (desde host)
curl -X POST http://localhost:8080/api/v1/peers/172.18.0.3:8081/connect
curl -X POST http://localhost:8080/api/v1/peers/172.18.0.4:8081/connect
```

## 💾 Persistencia de Datos

### Volumen nombrado (recomendado)

```bash
docker run -d \
  --name rust-bc-node \
  -v blockchain-data:/app/data \
  rust-bc:latest
```

### Bind mount (desarrollo)

```bash
docker run -d \
  --name rust-bc-node \
  -v $(pwd)/data:/app/data \
  rust-bc:latest
```

### Backup de datos

```bash
# Backup
docker run --rm \
  -v blockchain-data:/data \
  -v $(pwd):/backup \
  alpine tar czf /backup/blockchain-backup.tar.gz -C /data .

# Restore
docker run --rm \
  -v blockchain-data:/data \
  -v $(pwd):/backup \
  alpine tar xzf /backup/blockchain-backup.tar.gz -C /data
```

## 🔍 Monitoreo y Debugging

### Ver logs

```bash
# Logs en tiempo real
docker logs -f rust-bc-node

# Últimas 100 líneas
docker logs --tail 100 rust-bc-node

# Logs con timestamp
docker logs -f -t rust-bc-node
```

### Health Check

```bash
# Verificar salud del contenedor
docker ps

# Health check manual
curl http://localhost:8080/api/v1/health
```

### Ejecutar comandos dentro del contenedor

```bash
# Shell interactivo
docker exec -it rust-bc-node /bin/bash

# Ver base de datos
docker exec rust-bc-node ls -lh /app/data/
```

## 🚀 Deployment en Producción

### Docker Swarm

```yaml
# docker-stack.yml
version: '3.8'
services:
  rust-bc:
    image: rust-bc:latest
    deploy:
      replicas: 3
      update_config:
        parallelism: 1
        delay: 10s
    ports:
      - "8080:8080"
      - "8081:8081"
    volumes:
      - blockchain-data:/app/data
```

```bash
# Deploy
docker stack deploy -c docker-stack.yml blockchain
```

### Kubernetes

```yaml
# k8s-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rust-bc
spec:
  replicas: 3
  selector:
    matchLabels:
      app: rust-bc
  template:
    metadata:
      labels:
        app: rust-bc
    spec:
      containers:
      - name: rust-bc
        image: rust-bc:latest
        ports:
        - containerPort: 8080
        - containerPort: 8081
        env:
        - name: API_PORT
          value: "8080"
        - name: P2P_PORT
          value: "8081"
        volumeMounts:
        - name: data
          mountPath: /app/data
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: blockchain-data
```

## 🔒 Seguridad

### Usuario no-root

La imagen ya ejecuta como usuario no-root (`rustbc`, UID 1000).

### Redes aisladas

```bash
# Crear red aislada
docker network create --driver bridge blockchain-net

# Ejecutar en red aislada
docker run -d \
  --name rust-bc-node \
  --network blockchain-net \
  rust-bc:latest
```

### Secrets (para producción)

```bash
# Usar Docker secrets
echo "my-secret-key" | docker secret create api_key -

# En docker-compose.yml
services:
  rust-bc:
    secrets:
      - api_key
secrets:
  api_key:
    external: true
```

## 📊 Optimizaciones

### Build cache

```bash
# Usar BuildKit para mejor cache
DOCKER_BUILDKIT=1 docker build -t rust-bc:latest .
```

### Imagen multi-stage

El Dockerfile ya usa multi-stage build para minimizar tamaño final.

### Resource limits

```yaml
# docker-compose.yml
services:
  node1:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
        reservations:
          cpus: '1'
          memory: 1G
```

## 🐛 Troubleshooting

### Contenedor no inicia

```bash
# Ver logs de error
docker logs rust-bc-node

# Verificar configuración
docker inspect rust-bc-node
```

### Puerto ya en uso

```bash
# Cambiar puertos
docker run -d \
  -p 3000:8080 \
  -p 4000:8081 \
  rust-bc:latest
```

### Problemas de permisos

```bash
# Verificar usuario
docker exec rust-bc-node whoami

# Verificar permisos de volumen
docker exec rust-bc-node ls -la /app/data
```

## 📚 Recursos Adicionales

- [Docker Documentation](https://docs.docker.com/)
- [Docker Compose Documentation](https://docs.docker.com/compose/)
- [Rust Docker Best Practices](https://github.com/rust-lang/cargo/issues/2644)

## 🤝 Contribuir

Si encuentras problemas o mejoras para el Dockerfile, por favor abre un issue o PR.

