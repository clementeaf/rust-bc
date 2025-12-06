# 🔧 Instalación y Configuración de Rust

## 📋 Problema Detectado

**Error**: `zsh: command not found: cargo`

Esto significa que Rust/Cargo no está instalado o no está en tu PATH.

---

## 🚀 Solución: Instalar Rust

### Opción 1: Instalación Automática (Recomendado)

Rust se instala fácilmente con un script oficial:

```bash
# Ejecutar en tu terminal
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Este script:
- ✅ Instala Rust y Cargo automáticamente
- ✅ Configura el PATH en tu shell
- ✅ Instala las herramientas necesarias

**Después de la instalación:**
```bash
# Recargar la configuración del shell
source ~/.cargo/env

# O reiniciar la terminal

# Verificar instalación
cargo --version
rustc --version
```

---

### Opción 2: Verificar si ya está instalado

Si Rust ya está instalado pero no está en el PATH:

```bash
# 1. Verificar si existe
ls -la ~/.cargo/bin/cargo

# 2. Si existe, agregar al PATH manualmente
# Agregar esto a tu ~/.zshrc:
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc

# 3. Recargar configuración
source ~/.zshrc

# 4. Verificar
cargo --version
```

---

## 📝 Configuración del Shell (zsh)

Si Rust está instalado pero no funciona, agrega esto a tu `~/.zshrc`:

```bash
# Agregar al final de ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
```

Luego:
```bash
source ~/.zshrc
```

---

## ✅ Verificación de Instalación

Después de instalar/configurar, verifica:

```bash
# Verificar Cargo
cargo --version
# Debería mostrar: cargo 1.xx.x (xxxxx xxxx-xx-xx)

# Verificar Rust
rustc --version
# Debería mostrar: rustc 1.xx.x (xxxxx xxxx-xx-xx)

# Verificar Rustup
rustup --version
# Debería mostrar: rustup 1.xx.x (xxxxx xxxx-xx-xx)
```

---

## 🔧 Solución Rápida (Si ya está instalado)

Si Rust ya está instalado pero no funciona en esta sesión:

```bash
# Opción 1: Cargar entorno
source ~/.cargo/env

# Opción 2: Agregar al PATH manualmente
export PATH="$HOME/.cargo/bin:$PATH"

# Verificar
cargo --version
```

---

## 📚 Recursos Adicionales

- **Sitio oficial**: https://www.rust-lang.org/
- **Instalación**: https://www.rust-lang.org/tools/install
- **Documentación**: https://doc.rust-lang.org/

---

## 🎯 Después de Instalar Rust

Una vez que Rust esté instalado y funcionando:

1. **Compilar el proyecto**:
   ```bash
   cd /Users/clementefalcone/Desktop/personal/rust-bc
   cargo build --release
   ```

2. **Ejecutar el servidor**:
   ```bash
   cargo run 8080 8081 blockchain
   ```

3. **Ejecutar las pruebas** (en otra terminal):
   ```bash
   ./scripts/test_sistema_completo.sh
   ```

---

## ⚠️ Notas Importantes

- La instalación de Rust puede tardar varios minutos
- Requiere conexión a internet
- Instala automáticamente todas las herramientas necesarias
- No requiere permisos de administrador (instala en tu home)

---

**¿Necesitas ayuda?** Si tienes problemas con la instalación, consulta la documentación oficial o verifica que tu sistema cumpla con los requisitos.

