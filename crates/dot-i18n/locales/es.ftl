### Dot texts in Spanish.
### One line per text: `identifier = text`. Identifiers are never translated.
### `{ -app-name }` inserts the app name.

-app-name = Dot

## Window

window-title = { -app-name }

## App menu, in the macOS menu bar

menu-about = Acerca de { -app-name }
menu-services = Servicios
menu-hide = Ocultar { -app-name }
menu-hide-others = Ocultar otros
menu-show-all = Mostrar todo
menu-quit = Salir de { -app-name }

## Errors that stop Dot

error-event-loop = El bucle de eventos de { -app-name } no pudo ejecutarse.
error-window = { -system } se negó a abrir la ventana de { -app-name }.
error-surface = La ventana de { -app-name } no pudo conectarse a la tarjeta gráfica.
error-adapter = No se encontró ninguna tarjeta gráfica compatible.
error-device = No se pudo abrir la tarjeta gráfica.
error-unsupported-surface = La tarjeta gráfica no puede dibujar en la ventana de { -app-name }.
error-surface-lost = Se perdió la superficie de dibujo de { -app-name }.
error-text = { -app-name } no pudo dibujar su texto.

## Sign-in screen, shown while nothing connects Dot to Claude

sign-in-title = Conectarse a Claude
sign-in-intro = Elige cómo usa { -app-name } a Claude.
sign-in-claude-account = Cuenta de Claude
sign-in-claude-account-detail = tu plan Pro o Max, a través de Claude Code
sign-in-anthropic-account = Cuenta de Anthropic
sign-in-anthropic-account-detail = se paga por uso, Consola de Anthropic, a través de ant
sign-in-api-key = Clave de API
sign-in-api-key-detail = se paga por uso, Consola de Anthropic
sign-in-choose-hint = ↑↓ elegir · intro confirmar
sign-in-key-prompt = Pega tu clave de API de Anthropic.
sign-in-key-hint = { -command-key }+V pegar · intro conectar · esc volver
sign-in-checking-key = Comprobando la clave con Anthropic…
sign-in-waiting-for-claude-code = Iniciando sesión en tu navegador, a través de Claude Code…
sign-in-waiting-for-ant = Iniciando sesión en tu navegador, a través de ant…
sign-in-cancel-hint = esc cancelar
sign-in-key-malformed = Esta no es una clave de API de Anthropic: empieza por sk-ant-.
sign-in-key-rejected = Anthropic rechazó esta clave.
sign-in-anthropic-unreachable = Anthropic no responde. Revisa tu conexión y vuelve a intentarlo.
sign-in-claude-code-missing = Claude Code no está instalado. Instálalo y vuelve a intentarlo.
sign-in-claude-code-failed = El inicio de sesión a través de Claude Code no se completó.
sign-in-ant-missing = La herramienta ant de Anthropic no está instalada: brew install anthropics/tap/ant
sign-in-account-failed = El inicio de sesión en la cuenta de Anthropic a través de ant no se completó.
sign-in-not-connected = { -app-name } ya no está conectado a Claude.
sign-in-storage-failed = El llavero de { -system } denegó el acceso.
sign-in-crashed = { -app-name } falló al conectarse. Vuelve a intentarlo.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5 h
status-weekly-limit = 7 d
status-base = base
effort-low = bajo
effort-medium = medio
effort-high = alto
effort-extra-high = muy alto
effort-max = máx.

## Prompter, above the status bar

suggestion-explain-project = Explícame este proyecto
suggestion-fix-bug = Encuentra y corrige un error
suggestion-write-tests = Escribe las pruebas que faltan
suggestion-review-changes = Revisa mis últimos cambios
suggestion-simplify-code = Haz este código más simple
suggestion-find-startup = ¿Dónde arranca la app?
suggestion-summarize-commits = Resume los últimos commits
suggestion-run-tests = Ejecuta las pruebas y arregla lo que falle

## Key help, beside the reading line while nothing happens

help-send-key = intro
help-send = enviar
help-new-line-key = mayús+intro
help-new-line = nueva línea
help-pause-key = tab
help-pause = pausa
help-stop-key = esc
help-stop = detener
help-review-key = rueda
help-review = releer
help-switch-key = { -command-sign }↑ { -command-sign }↓
help-switch = cambiar de sesión
help-settings-key = { -command-sign } rueda
help-settings = ajustes

## Images, in the prompter and the conversation

image-label = imagen
image-pasted = imagen pegada
size-kilobytes = KB
size-megabytes = MB
decimal-separator = ,
unit-seconds = s
instrument-tokens = Tokens
instrument-time = Tiempo
instrument-agents = Agentes
unit-tokens-per-second = tok/s
unit-tokens = { $count ->
    [one] token
   *[other] tokens
}
state-thinking = Claude piensa
state-writing = Claude escribe
state-reading = Claude lee { $value }
state-searching = Claude busca { $value }
state-editing = Claude edita { $value }
state-creating = Claude crea { $value }
state-running = Claude ejecuta { $value }
state-browsing = Claude consulta { $value }
state-using = Claude usa { $value }
state-waiting-for-agents = { $count ->
    [one] Claude espera a 1 agente
   *[other] Claude espera a { $count } agentes
}
percent = { $value }{" "}%
state-paused = En pausa
state-queued = 1 prompt en espera
note-placeholder = una indicación para Claude, luego intro · tab reanuda
queued-after-the-reply = se enviará tras la respuesta
agents-at-work = { $count ->
    [one] 1 agente trabajando
   *[other] { $count } agentes en paralelo
}
agents-out-of-sight = { $count ->
    [one] 1 agente
   *[other] { $count } agentes
}
agent-started-by-claude = iniciado por Claude · { $value }
agent-view-placeholder = un agente trabajando no lee nada de lo que escribes aquí · esc vuelve
state-agent-working = { $value } · trabajando
state-agent-done = { $value } · terminado
agent-read = { $count ->
    [one] 1 archivo leído
   *[other] { $count } archivos leídos
}
agent-searched = { $count ->
    [one] 1 búsqueda
   *[other] { $count } búsquedas
}
agent-edited = { $count ->
    [one] 1 archivo modificado
   *[other] { $count } archivos modificados
}
agent-created = { $count ->
    [one] 1 archivo creado
   *[other] { $count } archivos creados
}
agent-ran = { $count ->
    [one] 1 comando
   *[other] { $count } comandos
}
agent-browsed = { $count ->
    [one] 1 página leída
   *[other] { $count } páginas leídas
}
agent-delegated = { $count ->
    [one] 1 agente iniciado
   *[other] { $count } agentes iniciados
}
agent-used = { $count ->
    [one] 1 herramienta
   *[other] { $count } herramientas
}
session-name = sesión { $value }
cost-dollars = { $value }{" "}US$
quote-open = «
quote-close = »
image-unsupported = Claude no lee este tipo de imagen: PNG, JPEG, GIF o WebP.
image-too-large = Esta imagen supera los 5 MB, el límite de Claude.
image-too-many = Un mensaje lleva 20 imágenes como máximo, 20 MB en total.

## Conversation, when a reply does not end as Claude finished it

reply-stopped = Respuesta detenida.
reply-unauthorized = Anthropic rechazó la clave o la cuenta.
reply-rejected = Anthropic rechazó la solicitud. Si la conversación es demasiado larga, { -command-key }+N empieza una nueva.
reply-rate-limited = Se alcanzó el límite de uso. Vuelve a intentarlo más tarde.
reply-overloaded = Claude está sobrecargado. Vuelve a intentarlo en un momento.
reply-unreachable = Anthropic no responde. Revisa tu conexión y vuelve a intentarlo.
reply-declined = Claude se negó a responder a esta solicitud.
reply-claude-code-failed = Claude Code no pudo responder.
reply-oversized = Una parte de la respuesta supera los 4 MB, más de lo que { -app-name } lee: la respuesta se corta y no se guarda nada de ella.
reply-crashed = { -app-name } falló durante esta respuesta: la conversación empieza de cero.
conversation-restarted = Nueva conversación: Claude ya no ve lo anterior.
conversation-compacted = Conversación resumida para Claude:

## Conversation, the tools Claude uses: what each one does, in front of what it works on

tool-read = leer
tool-write = escribir
tool-edit = editar
tool-run = $
tool-find-files = encontrar
tool-search-text = buscar
tool-fetch = abrir
tool-search-web = web
tool-delegate = delegar
tool-other = herramienta

## Conversation, how a tool ended, after what it worked on

tool-failed = falló
tool-refused = rechazado
tool-stopped = detenido
tool-lines = { $count ->
    [one] línea
   *[other] líneas
}
tool-files = { $count ->
    [one] archivo
   *[other] archivos
}
tool-matches = { $count ->
    [one] resultado
   *[other] resultados
}
tool-exit = código
gutter-lines = lín.
code-new = nuevo
code-fold = plegar
editor-by-claude = Claude:
editor-created-by-claude = creado por Claude ·
editor-by-you = tú:
editor-to-claude = la versión de Claude
editor-to-before = antes de Claude
editor-to-yours = tus cambios
editor-back = volver
editor-saved = guardado con cada tecla
editor-not-saved = sin guardar
editor-outside = Este archivo está fuera de la carpeta de la sesión: Dot no lo abre.
editor-not-a-file = Esto no es un archivo.
editor-too-large = Este archivo supera 1 MiB: demasiado grande para retocarlo aquí.
editor-not-text = Este archivo no es texto.
editor-failed = Este archivo no se abre.
editor-lost = Dot no pudo guardar { $value }: tus últimos retoques no están en él.

## Prompter, when Claude asks before it goes ahead

permission-run = ¿Ejecutar este comando?
permission-change = ¿Modificar este archivo?
permission-change-outside = ¿Modificar este archivo, fuera del proyecto?
permission-read-outside = ¿Leer esto, fuera del proyecto?
permission-fetch = ¿Abrir esta página?
permission-search-web = ¿Buscar en la web?
permission-use = ¿Usar esta herramienta?
permission-allow = intro permitir
permission-deny = esc rechazar
grant-command = tab siempre este comando
grant-changes = tab todos los archivos del proyecto
grant-file = tab siempre este
grant-host = tab siempre este sitio
grant-search-web = tab siempre la web
grant-tool = tab siempre esta herramienta

## Settings
settings-title = Ajustes
settings-saved = se guarda con cada cambio
settings-unsaved = No se pudieron guardar los ajustes.
settings-choose = elegir
settings-change = cambiar
settings-reset = predeterminado
settings-close = cerrar
settings-default = Predeterminado
settings-rank = Ajuste
settings-conversation = Conversación
settings-display = Pantalla
setting-model = Modelo
setting-model-hint = El modelo que responde, desde el próximo envío.
setting-effort = Esfuerzo
setting-effort-hint = Cuánto piensa Claude antes de responder. Más alto: más lento y más caro.
setting-summaries = Resumen automático
setting-summaries-hint = Dot resume la conversación cuando releerla cuesta más que un resumen.
summaries-when-they-pay = cuando compensa
summaries-left-to-claude = contexto lleno
setting-permissions = Acciones de Claude
setting-permissions-hint = Leer, escribir, ejecutar un comando: sin preguntar, o con una pregunta cada vez.
permissions-bypass = permitidas de entrada
permissions-ask = preguntadas
setting-color = Color
setting-color-hint = El color de esta sesión: lo que cambia ahora mismo, su tarjeta, las herramientas y el indicador. Cada sesión nueva toma un color que ninguna otra tiene.
hue-orange = naranja
hue-red = rojo
hue-amber = ámbar
hue-green = verde
hue-cyan = cian
hue-blue = azul
hue-violet = violeta
hue-pink = rosa
setting-language = Idioma
setting-language-hint = El idioma de todos los textos de Dot. Claude responde en el idioma de tu pregunta.
setting-column = Ancho de columna
setting-column-hint = El máximo de caracteres por línea. Una ventana estrecha muestra menos.
column-characters = { $value } col
setting-reading-line = Línea de lectura
setting-reading-line-hint = La altura de la franja donde escribes. Se mueve al instante.
reading-line-middle = al centro
reading-line-third = a un tercio
setting-caret = Cursor _
setting-caret-hint = El _ de la línea de escritura y de la respuesta.
caret-blinking = parpadea
caret-steady = fijo
setting-sessions-column = Columna de sesiones
setting-sessions-column-hint = auto: se muestra en cuanto la ventana es lo bastante ancha.
sessions-column-auto = auto
sessions-column-never = nunca
setting-state-colors = Colores de estado
setting-state-colors-hint = ✓ en verde, ✗ en rojo, detenido en ámbar; el indicador del contexto en el color de la sesión cuando sube, y desde el 80 %.
setting-tool-colors = Colores de herramientas
setting-tool-colors-hint = El glifo de cada herramienta (▤ ⌕ ✎ ⌘ ⧉), en el color de la sesión.
toggle-on = sí
toggle-off = no
setting-size = Tamaño
setting-size-hint = Tamaño de todo lo que Dot dibuja: texto, márgenes, barra de estado, ajustes; 80 % era el tamaño anterior.
size-percent = { $value } %
settings-outputs = Salidas de comandos
setting-output-cleaning = Limpieza del terminal
setting-output-cleaning-hint = Colores y líneas redibujadas en su sitio (barras de progreso) quitados de lo que Claude lee; una línea redibujada se cuenta una vez.
setting-output-rewriting = Reescritura sin pérdida
setting-output-rewriting-hint = Rutas repetidas escritas una vez, listas de archivos plegadas, JSON sin espacios, árboles, tablas, git status y git log más cortos; no se pierde nada.
setting-output-progress = Progreso contado
setting-output-progress-hint = Progreso de herramientas conocidas (compilaciones, instalaciones) y pruebas superadas contados: [+12 lines: build progress]; los errores quedan enteros.
setting-contents-cut = Recorte de archivos y diffs
setting-contents-cut-hint = Un archivo o un diff más largo que el primer tamaño se reduce hacia el segundo, guardado entero en /tmp/dot-whole; medido: 10 → 6 KB.
setting-outputs-cut = Recorte de otras salidas
setting-outputs-cut-hint = Cualquier otra salida más larga que el primer tamaño se reduce hacia el segundo si el recorte quita 4 KB o más, guardada entera en /tmp/dot-whole; medido: 7 → 3 KB.
cut-never = nunca
cut-bounds = { $value } KB
settings-connection = Conexión
setting-folder = Carpeta
setting-folder-hint = intro abre el selector de carpetas; una sesión nueva se abre en la elegida.
folder-none = ninguna
folder-picker-prompt = Abrir una sesión aquí
setting-account = Cuenta
setting-account-hint = intro cambia la conexión: plan de Claude, cuenta de Anthropic o clave de API.
account-none = ninguna
setting-api-key = Clave de API
setting-api-key-hint = intro para pegar una, intro otra vez para comprobarla. Va al llavero, nunca en texto plano.
api-key-none = ninguna
api-key-kept = en el llavero
api-key-checking = comprobando…
setting-sign-out = Cerrar sesión
setting-sign-out-hint = intro y luego intro otra vez para confirmar. Dot olvida su conexión; Claude Code y ant siguen conectados.
sign-out-confirm = intro otra vez para confirmar
selection-label = Selección
selection-characters = { $value } car.
selection-lines = { $value } líneas
selection-copy = copiar
selection-copied = copiado
