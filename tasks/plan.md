# Plano de implementação — Neo AutoClicker

## Direção

Aplicativo nativo para Windows, em português, com visual escuro de estação de controle moderna. A interface destaca a execução, reúne o ritmo numa coluna lateral e mantém destino, repetição, pontos e atalhos no painel principal. Um indicador discreto pulsa somente durante a execução.

## Entregue

- [x] Preservar perfis, cursor atual, posição fixa, sequência, repetição, limites, overlay e métricas honestas.
- [x] Remover a seleção Normal / Alta precisão / Sem limite e usar uma agenda única para CPS e intervalo.
- [x] Trocar atalhos por gravação direta ao clicar no chip; acumular teclas e salvar após soltá-las.
- [x] Configurar atalhos para iniciar/parar, capturar no cursor, exibir/ocultar overlay, adicionar coordenadas e limpar pontos.
- [x] Pedir confirmação para limpar pontos e permitir editar as coordenadas antes de adicioná-las.
- [x] Renomear a aplicação para Neo AutoClicker e preservar a leitura do diretório legado de perfis.
- [x] Atualizar a identidade escura, workflow, documentação e nomes do executável/artefato.
- [x] Validar formatação, lint, testes e build release Linux.

## Verificação pendente

- [ ] Compilar e gerar o artefato x64 MSVC pelo workflow do GitHub.
- [ ] Conferir interação de atalhos, overlay, DPI e multi-monitor em um desktop Windows.

O ambiente de desenvolvimento tem apenas o target Linux instalado. A configuração de build Windows está em `.github/workflows/windows.yml`.
