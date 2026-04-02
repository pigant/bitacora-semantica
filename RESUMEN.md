RESUMEN

Propósito
- Interfaz de terminal (TUI) para capturar "Log Diario" y producir una propuesta de comando ejecutable de Mulch (comando `ml record`) a partir de los datos ingresados por el usuario.

Audiencia
- Operadores o desarrolladores que mantengan una base de conocimiento Mulch y quieran convertir notas/observaciones en registros formales mediante un flujo asistido.

Requisitos de ejecución
- Entorno con las herramientas CLI requeridas instaladas en PATH: `pi` (agente RPC) y `ml` (herramienta Mulch).
- Un repositorio inicializado con Mulch (`.mulch/`) cuando se desea que las sugerencias consulten la base de conocimiento local.

Estructura general del proyecto (alto nivel)
- ui/: implementación de la TUI y de los helpers de inferencia.
  - main.rs (entrada y bucle de la aplicación)
  - ui.rs (renderizado y disposición de paneles)
  - state.rs (estado de la aplicación y modelo del formulario)
  - inference.rs (lógica que invoca `ml` y `pi` para generar sugerencias)
  - pi_rpc.rs (helpers para comunicación RPC con `pi`)
- tests/ y fixtures/ (casos de prueba y plantillas de prompt para testing)

Flujos principales (qué hace la aplicación)
- Formulario de entrada: el usuario completa campos como Título (Title), Dominio (Domain), Fecha, Participantes, Archivos (Files), Etiquetas (Tags), Descripción (Description) y un espacio para solicitar una sugerencia de registro.

- Sugerir registro (comando ml record): acción explícita del usuario que solicita al agente generar una línea de comando ejecutable del tipo `ml record ...` que capture el contenido del formulario. El flujo es:
  1. El cliente captura contexto local necesario (ej.: salida de `ml prime`) y construye un prompt.
  2. Se invoca `pi` en modo RPC con ese prompt y se espera la respuesta final del agente.
  3. Si la respuesta no contiene la línea de comando esperada, la app puede enviar un follow‑up en la misma sesión solicitando estrictamente la línea de comando.
  4. La aplicación presenta la línea de comando sugerida al usuario para revisión (no la ejecuta automáticamente).
  5. Si el agente no produce un comando válido, la aplicación construye un comando de fallback a partir de los campos del formulario para que el usuario lo revise.

Interacción y usabilidad (resumen de la UX)
- Navegación por campos con teclado; hay un campo especial "Sugerir registro" que actúa como un botón (acción explícita) y también permite edición manual del razonamiento.
- La vista de "Preview" (panel derecho) está dedicada a mostrar únicamente la sugerencia del comando `ml record` (o un spinner mientras se genera). No autofill automático de campos relacionales: el usuario decide aplicar/ejecutar el comando.
- La aplicación evita entrar en modo edición accidental del "botón" para no confundir la navegación con la invocación de la acción.

Concurrencia y robustez (comportamiento observable)
- Las tareas que llaman a `pi` y `ml` se ejecutan en hilos de fondo para no bloquear la UI.
- La UI escucha resultados de estos workers y actualiza inmediatamente cuando los resultados llegan (sin requerir interacción adicional).
- Se implementan medidas de resiliencia: extracción de la respuesta final del agente, follow‑up en caso de formato no esperado, y fallback generado localmente si no hay resultado útil.

Registros y diagnósticos (ruta de logs)
- La aplicación guarda trazas y eventos relevantes para depuración en logs/*.log (archivos con nombres descriptivos, por ejemplo logs relacionados con inferencia y sugerencias). Consultar esos archivos cuando la sugerencia no aparece o hay errores externos.

Seguridad y comportamiento de ejecución
- La aplicación no ejecuta automáticamente el comando `ml record` sin confirmación explícita del usuario. La sugerencia se presenta para revisión y aprobación.

Qué está fuera del alcance actual
- Ejecución automática de los comandos sugeridos sin confirmación.
- Integraciones GUI fuera de terminal.

Recomendaciones siguientes (posibles mejoras)
- Añadir un modal de confirmación con opciones "Copiar" y "Ejecutar" para la sugerencia de `ml record` y registrar la salida de la ejecución en un archivo de log.
- Mejorar el sanitizado y escape de argumentos en la línea de comando sugerida para evitar problemas con comillas/espacios.
- Enriquecer la heurística de coincidencias locales (bigramas, stopwords) para obtener referencias más precisas en prompts.
- Añadir pruebas de integración específicas para el flujo de `suggest_ml_record` usando fixtures reproducibles.

Cómo empezar a usarla (pasos mínimos)
1. Asegurarse de tener `pi` y `ml` disponibles en PATH y, si se desea contexto local, un repositorio con `.mulch/`.
2. Ejecutar la aplicación de la carpeta ui (por ejemplo, con cargo run en el subdirectorio si se tiene Rust instalado).
3. Rellenar los campos necesarios (Title y Domain son requeridos para generar el registro).
4. Focalizar "Sugerir registro" y activar la acción (Enter); revisar la línea mostrada en el panel Preview.
5. Revisar los logs en /tmp si la sugerencia no aparece o hay errores.
