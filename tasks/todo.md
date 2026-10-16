# Lista de tarefas

## Base funcional
- [x] Criar crate Rust 2024, janela egui, modelos, persistência, ícone e README. fmt, clippy, testes e build release Linux passaram.
- [x] Implementar motor cancelável, contadores e backend Win32 `SendInput`. Testes do modelo/motor passaram; backend Windows ainda precisa de compilação e teste no host.

## Controles e destinos
- [x] Adicionar controles de execução e velocidade e destinos atual/fixo/sequência. Regras puras e build local passaram.
- [x] Adicionar hotkeys globais, captura, lista e edição de posições. Teste manual Win32 pendente.

## Overlay e perfis
- [x] Adicionar overlay transparente multi-monitor e controle de exibição. Teste manual no desktop Windows pendente.
- [x] Adicionar perfis nomeados e persistência robusta. Testes de ciclo e serialização passaram.

## Fechamento
- [x] Corrigir lint/build Linux e documentar limites de plataforma. fmt, clippy, testes e build release passaram.
- [ ] Produzir `RustAutoClicker.exe` e verificar interações no Windows. `cl.exe` está montado no disco Windows, mas WSL não executa binários Windows; faltam `rc.exe`/cabeçalhos do SDK e o target Rust MSVC. Instalação requer autorização.
