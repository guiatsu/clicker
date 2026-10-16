# Plano de implementação: RustAutoClicker

## Visão geral

Criar um aplicativo desktop único em Rust com interface egui, motor isolado da UI, controles de velocidade e destino, captura e lista de posições, perfis JSON e documentação. O código terá uma camada Win32 para interação real com o Windows e camadas portáveis para validação de modelo e persistência no WSL.

## Decisões de arquitetura

- `eframe`/`egui` para a janela e os controles; `serde`/`serde_json` para persistência.
- `std::thread` e canais da biblioteca padrão para manter o loop de cliques fora da UI e interrompível.
- `windows-sys` somente em Windows para `SendInput`, coordenadas e integração global com o desktop.
- Um único perfil JSON por usuário, com perfis nomeados; nenhuma dependência de serviço ou processo externo.
- O motor vai medir os eventos que entregou ao sistema, sem afirmar que aplicações receptoras processaram todos eles.
- Este workspace é WSL2. O `cl.exe` existe no disco Windows, mas WSL não consegue iniciar executáveis Windows nesta sessão; não há target Rust para Windows, `rc.exe` ou cabeçalhos do SDK. PowerShell também falha pelo mesmo caminho. A geração do `.exe` e os testes de desktop dependem de autorização e de toolchain Windows acessível.

## Tarefas

### Fase 1: Base funcional
- [x] Criar crate Rust 2024, janela egui, modelos de configuração, persistência resiliente e README.
- [x] Implementar conversões CPS/intervalo, validação e regras puras para perfis e posições.
- [x] Implementar motor cancelável com métricas e backend Win32 de `SendInput`.

### Checkpoint: Base
- [x] `cargo fmt`, `cargo clippy`, `cargo test` e `cargo build --release` no ambiente Linux disponível.
- [x] O app inicia parado; o código não inicia cliques automaticamente.

### Fase 2: Controles e destinos
- [x] Criar UI para velocidade, botão, duração, repetição e estado do motor.
- [x] Implementar posição atual, fixa e sequência ordenada com pausas por posição.
- [x] Implementar polling de atalhos globais Win32 com borda de pressionamento, captura e validação de conflito.

### Checkpoint: Fluxos principais
- [ ] Verificar controles e validações localmente; verificar hotkeys e execução no Windows quando houver desktop Windows disponível.

### Fase 3: Posições, overlay e perfis
- [x] Criar edição, ordenação, remoção e limpeza confirmada da lista de posições.
- [x] Implementar overlay transparente, topmost e click-through no espaço virtual multi-monitor.
- [x] Criar interface de perfil para selecionar, criar, renomear, duplicar e excluir.
- [x] Incluir ícone e metadados de versão no recurso Windows.

### Checkpoint: Aplicativo completo
- [x] Testar ciclo de perfis, serialização JSON e motor; confirmar no código que a inicialização não clica.
- [ ] Compilar e testar interação no Windows; gerar `RustAutoClicker.exe` após autorização e acesso ao toolchain Windows.

## Riscos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| WSL sem target MSVC, sem `rc.exe`/cabeçalhos do Windows SDK e sem interoperabilidade para executar `cl.exe`/PowerShell | Alto: sem `.exe` nem testes manuais de desktop | Implementar e validar partes portáveis; pedir autorização antes de instalar componentes; documentar bloqueio com precisão |
| Atalhos e overlay dependem de APIs Windows e monitoramento DPI | Alto | Isolar a camada Win32 e documentar os testes pendentes no host Windows |
| CPS extremo excede precisão do scheduler ou do receptor | Médio | Usar relógio monotônico, medir eventos enviados e sinalizar limites na interface |

## Questões abertas

- A instalação do target Rust Windows e de Build Tools/Windows SDK no host requer autorização explícita, conforme a solicitação original. Nenhum instalador foi executado.
