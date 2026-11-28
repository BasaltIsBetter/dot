### Dot texts in Brazilian Portuguese.
### One line per text: `identifier = text`. Identifiers are never translated.
### `{ -app-name }` inserts the app name.

-app-name = Dot

## Window

window-title = { -app-name }

## App menu, in the macOS menu bar

menu-about = Sobre o { -app-name }
menu-services = Serviços
menu-hide = Ocultar { -app-name }
menu-hide-others = Ocultar Outros
menu-show-all = Mostrar Tudo
menu-quit = Encerrar { -app-name }

## Errors that stop Dot

error-event-loop = O laço de eventos do { -app-name } não pôde ser executado.
error-window = O { -system } se recusou a abrir a janela do { -app-name }.
error-surface = Não foi possível ligar a janela do { -app-name } à placa de vídeo.
error-adapter = Nenhuma placa de vídeo compatível foi encontrada.
error-device = Não foi possível abrir a placa de vídeo.
error-unsupported-surface = A placa de vídeo não consegue desenhar na janela do { -app-name }.
error-surface-lost = A superfície de desenho do { -app-name } foi perdida.
error-text = O { -app-name } não conseguiu desenhar seu texto.

## Sign-in screen, shown while nothing connects Dot to Claude

sign-in-title = Conectar ao Claude
sign-in-intro = Escolha como o { -app-name } usa o Claude.
sign-in-claude-account = Conta Claude
sign-in-claude-account-detail = seu plano Pro ou Max, pelo Claude Code
sign-in-anthropic-account = Conta Anthropic
sign-in-anthropic-account-detail = cobrado por uso, Console da Anthropic, pelo ant
sign-in-api-key = Chave de API
sign-in-api-key-detail = cobrada por uso, Console da Anthropic
sign-in-choose-hint = ↑↓ escolher · enter confirmar
sign-in-key-prompt = Cole sua chave de API da Anthropic.
sign-in-key-hint = { -command-key }+V colar · enter conectar · esc voltar
sign-in-checking-key = Verificando a chave com a Anthropic…
sign-in-waiting-for-claude-code = Entrando pelo navegador, pelo Claude Code…
sign-in-waiting-for-ant = Entrando pelo navegador, pelo ant…
sign-in-cancel-hint = esc cancelar
sign-in-key-malformed = Esta não é uma chave de API da Anthropic: ela começa com sk-ant-.
sign-in-key-rejected = A Anthropic recusou esta chave.
sign-in-anthropic-unreachable = A Anthropic não está respondendo. Verifique sua conexão e tente de novo.
sign-in-claude-code-missing = O Claude Code não está instalado. Instale-o e tente de novo.
sign-in-claude-code-failed = O login pelo Claude Code não foi concluído.
sign-in-ant-missing = A ferramenta ant da Anthropic não está instalada: brew install anthropics/tap/ant
sign-in-account-failed = O login na conta Anthropic pelo ant não foi concluído.
sign-in-not-connected = O { -app-name } não está mais conectado ao Claude.
sign-in-storage-failed = O chaveiro do { -system } negou o acesso.
sign-in-crashed = O { -app-name } travou ao conectar. Tente de novo.

## Status bar, at the bottom of a connected window

status-five-hour-limit = 5 h
status-weekly-limit = 7 d
status-base = base
effort-low = baixo
effort-medium = médio
effort-high = alto
effort-extra-high = muito alto
effort-max = máx.

## Prompter, above the status bar

suggestion-explain-project = Explique este projeto para mim
suggestion-fix-bug = Encontre e corrija um bug
suggestion-write-tests = Escreva os testes que faltam
suggestion-review-changes = Revise minhas últimas mudanças
suggestion-simplify-code = Deixe este código mais simples
suggestion-find-startup = Onde o app é iniciado?
suggestion-summarize-commits = Resuma os commits recentes
suggestion-run-tests = Rode os testes e corrija o que quebrar

## Key help, beside the reading line while nothing happens

help-send-key = enter
help-send = enviar
help-new-line-key = shift+enter
help-new-line = nova linha
help-pause-key = tab
help-pause = pausar
help-stop-key = esc
help-stop = parar
help-review-key = rolagem
help-review = reler
help-switch-key = { -command-sign }↑ { -command-sign }↓
help-switch = trocar de sessão
help-settings-key = { -command-sign } rolagem
help-settings = ajustes

## Images, in the prompter and the conversation

image-label = imagem
image-pasted = imagem colada
size-kilobytes = KB
size-megabytes = MB
decimal-separator = ,
unit-seconds = s
instrument-tokens = Tokens
instrument-time = Tempo
instrument-agents = Agentes
unit-tokens-per-second = tok/s
unit-tokens = { $count ->
    [one] token
   *[other] tokens
}
state-thinking = Claude está pensando
state-writing = Claude está escrevendo
state-reading = Claude lê { $value }
state-searching = Claude busca { $value }
state-editing = Claude edita { $value }
state-creating = Claude cria { $value }
state-running = Claude executa { $value }
state-browsing = Claude consulta { $value }
state-using = Claude usa { $value }
state-waiting-for-agents = { $count ->
    [one] Claude espera 1 agente
   *[other] Claude espera { $count } agentes
}
percent = { $value }%
state-paused = Em pausa
state-queued = 1 prompt na fila
note-placeholder = uma indicação para o Claude, depois enter · tab retoma
queued-after-the-reply = enviado após a resposta
agents-at-work = { $count ->
    [one] 1 agente trabalhando
   *[other] { $count } agentes em paralelo
}
agents-out-of-sight = { $count ->
    [one] 1 agente
   *[other] { $count } agentes
}
agent-started-by-claude = iniciado pelo Claude · { $value }
agent-view-placeholder = um agente trabalhando não lê nada do que você digita aqui · esc volta
state-agent-working = { $value } · trabalhando
state-agent-done = { $value } · concluído
agent-read = { $count ->
    [one] 1 arquivo lido
   *[other] { $count } arquivos lidos
}
agent-searched = { $count ->
    [one] 1 busca
   *[other] { $count } buscas
}
agent-edited = { $count ->
    [one] 1 arquivo alterado
   *[other] { $count } arquivos alterados
}
agent-created = { $count ->
    [one] 1 arquivo criado
   *[other] { $count } arquivos criados
}
agent-ran = { $count ->
    [one] 1 comando
   *[other] { $count } comandos
}
agent-browsed = { $count ->
    [one] 1 página lida
   *[other] { $count } páginas lidas
}
agent-delegated = { $count ->
    [one] 1 agente iniciado
   *[other] { $count } agentes iniciados
}
agent-used = { $count ->
    [one] 1 ferramenta
   *[other] { $count } ferramentas
}
session-name = sessão { $value }
cost-dollars = US${" "}{ $value }
quote-open = “
quote-close = ”
image-unsupported = O Claude não lê este tipo de imagem: PNG, JPEG, GIF ou WebP.
image-too-large = Esta imagem passa de 5 MB, o limite do Claude.
image-too-many = Uma mensagem leva no máximo 20 imagens, 20 MB no total.

## Conversation, when a reply does not end as Claude finished it

reply-stopped = Resposta interrompida.
reply-unauthorized = A Anthropic recusou a chave ou a conta.
reply-rejected = A Anthropic recusou o pedido. Se a conversa estiver longa demais, { -command-key }+N começa uma nova.
reply-rate-limited = Limite de uso atingido. Tente de novo mais tarde.
reply-overloaded = O Claude está sobrecarregado. Tente de novo em instantes.
reply-unreachable = A Anthropic não está respondendo. Verifique sua conexão e tente de novo.
reply-declined = O Claude se recusou a responder a este pedido.
reply-claude-code-failed = O Claude Code não conseguiu responder.
reply-oversized = Uma parte da resposta passa de 4 MB, mais do que o { -app-name } lê: a resposta é cortada e nada dela é guardado.
reply-crashed = O { -app-name } travou durante esta resposta: a conversa recomeça do zero.
conversation-restarted = Nova conversa: o Claude não vê mais o que veio antes.
conversation-compacted = Conversa resumida para o Claude:

## Conversation, the tools Claude uses: what each one does, in front of what it works on

tool-read = ler
tool-write = escrever
tool-edit = editar
tool-run = $
tool-find-files = encontrar
tool-search-text = buscar
tool-fetch = abrir
tool-search-web = web
tool-delegate = delegar
tool-other = ferramenta

## Conversation, how a tool ended, after what it worked on

tool-failed = falhou
tool-refused = recusado
tool-stopped = interrompido
tool-lines = { $count ->
    [one] linha
   *[other] linhas
}
tool-files = { $count ->
    [one] arquivo
   *[other] arquivos
}
tool-matches = { $count ->
    [one] resultado
   *[other] resultados
}
tool-exit = código
gutter-lines = lin.
code-new = novo
code-fold = recolher
editor-by-claude = Claude:
editor-created-by-claude = criado pelo Claude ·
editor-by-you = você:
editor-to-claude = a versão do Claude
editor-to-before = antes do Claude
editor-to-yours = suas alterações
editor-back = voltar
editor-saved = salvo a cada tecla
editor-not-saved = não salvo
editor-outside = Este arquivo está fora da pasta da sessão: o Dot não o abre.
editor-not-a-file = Isto não é um arquivo.
editor-too-large = Este arquivo passa de 1 MiB: grande demais para ser editado aqui.
editor-not-text = Este arquivo não é texto.
editor-failed = Este arquivo não abre.
editor-lost = O Dot não conseguiu salvar { $value }: suas últimas edições não estão nele.

## Prompter, when Claude asks before it goes ahead

permission-run = Executar este comando?
permission-change = Alterar este arquivo?
permission-change-outside = Alterar este arquivo, fora do projeto?
permission-read-outside = Ler isto, fora do projeto?
permission-fetch = Abrir esta página?
permission-search-web = Buscar na web?
permission-use = Usar esta ferramenta?
permission-allow = enter permitir
permission-deny = esc recusar
grant-command = tab sempre este comando
grant-changes = tab todos os arquivos do projeto
grant-file = tab sempre este
grant-host = tab sempre este site
grant-search-web = tab sempre a web
grant-tool = tab sempre esta ferramenta

## Settings
settings-title = Ajustes
settings-saved = salvo a cada mudança
settings-unsaved = Não foi possível salvar os ajustes.
settings-choose = escolher
settings-change = mudar
settings-reset = padrão
settings-close = fechar
settings-default = Padrão
settings-rank = Ajuste
settings-conversation = Conversa
settings-display = Tela
setting-model = Modelo
setting-model-hint = O modelo que responde nesta sessão, a partir do próximo envio; uma sessão nova começa com o mesmo.
setting-effort = Esforço
setting-effort-hint = Quanto o Claude pensa nesta sessão antes de responder. Mais alto: mais lento e mais caro.
setting-summaries = Resumo automático
setting-summaries-hint = O Dot resume a conversa quando relê-la custa mais do que um resumo.
summaries-when-they-pay = quando compensa
summaries-left-to-claude = contexto cheio
setting-permissions = Ações do Claude
setting-permissions-hint = Ler, escrever, executar um comando: sem perguntar, ou com uma pergunta a cada vez.
permissions-bypass = permitidas de cara
permissions-ask = perguntadas
setting-color = Cor
setting-color-hint = A cor desta sessão: o que muda neste momento, seu cartão, as ferramentas e o medidor. Cada sessão nova pega uma cor que nenhuma outra tem.
hue-orange = laranja
hue-red = vermelho
hue-amber = âmbar
hue-green = verde
hue-cyan = ciano
hue-blue = azul
hue-violet = violeta
hue-pink = rosa
setting-language = Idioma
setting-language-hint = O idioma de todos os textos do Dot. O Claude responde no idioma da sua pergunta.
setting-column = Largura da coluna
setting-column-hint = O máximo de caracteres por linha. Uma janela estreita mostra menos.
column-characters = { $value } col
setting-reading-line = Linha de leitura
setting-reading-line-hint = A altura da faixa onde você escreve. Ela se move na hora.
reading-line-middle = no meio
reading-line-third = a um terço
setting-caret = Cursor _
setting-caret-hint = O _ da linha de escrita e da resposta.
caret-blinking = pisca
caret-steady = fixo
setting-sessions-column = Coluna de sessões
setting-sessions-column-hint = auto: aparece assim que a janela fica larga o bastante.
sessions-column-auto = auto
sessions-column-never = nunca
setting-state-colors = Cores de estado
setting-state-colors-hint = ✓ em verde, ✗ em vermelho, interrompido em âmbar; o medidor de contexto na cor da sessão quando sobe, e a partir de 80%.
setting-tool-colors = Cores das ferramentas
setting-tool-colors-hint = O glifo de cada ferramenta (▤ ⌕ ✎ ⌘ ⧉), na cor da sessão.
toggle-on = sim
toggle-off = não
setting-size = Tamanho
setting-size-hint = Tamanho de tudo o que o Dot desenha: texto, margens, barra de status, ajustes; 80% era o tamanho anterior.
size-percent = { $value }%
settings-outputs = Saídas de comandos
setting-output-cleaning = Limpeza do terminal
setting-output-cleaning-hint = Cores e linhas redesenhadas no lugar (barras de progresso) retiradas do que o Claude lê; uma linha redesenhada é contada uma vez.
setting-output-rewriting = Reescrita sem perda
setting-output-rewriting-hint = Caminhos repetidos escritos uma vez, listas de arquivos recolhidas, JSON sem espaços, árvores, tabelas, git status e git log mais curtos; nada se perde.
setting-output-progress = Progresso contado
setting-output-progress-hint = Progresso de ferramentas conhecidas (builds, instalações) e testes aprovados contados: [+12 lines: build progress]; os erros ficam inteiros.
setting-contents-cut = Corte de arquivos e diffs
setting-contents-cut-hint = Um arquivo ou diff maior que o primeiro tamanho é reduzido para o segundo, guardado inteiro em /tmp/dot-whole; medido: 10 → 6 KB.
setting-outputs-cut = Corte de outras saídas
setting-outputs-cut-hint = Qualquer outra saída maior que o primeiro tamanho é reduzida para o segundo se o corte tirar 4 KB ou mais, guardada inteira em /tmp/dot-whole; medido: 7 → 3 KB.
cut-never = nunca
cut-bounds = { $value } KB
settings-connection = Conexão
setting-folder = Pasta
setting-folder-hint = enter abre o seletor de pastas; uma sessão nova abre na escolhida.
folder-none = nenhuma
folder-picker-prompt = Abrir uma sessão aqui
setting-account = Conta
setting-account-hint = enter muda a conexão: plano Claude, conta Anthropic ou chave de API.
account-none = nenhuma
setting-api-key = Chave de API
setting-api-key-hint = enter para colar uma, enter de novo para verificá-la. Ela vai para o chaveiro, nunca em texto puro.
api-key-none = nenhuma
api-key-kept = no chaveiro
api-key-checking = verificando…
setting-sign-out = Sair
setting-sign-out-hint = enter e depois enter de novo para confirmar. O Dot esquece sua conexão; o Claude Code e o ant continuam conectados.
sign-out-confirm = enter de novo para confirmar
selection-label = Seleção
selection-characters = { $value } car.
selection-lines = { $value } linhas
selection-copy = copiar
selection-copied = copiado
