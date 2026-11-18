# Alertas de resets extras do CC Switch

Complemento local para macOS integrado ao CC Switch 4.0.35. A integração usa o gerenciador OAuth já existente no CC Switch, que continua sendo o único responsável por renovar os logins. Não exporta tokens para outro aplicativo e não usa nem resgata resets.

## Funcionamento

Todas as contas ChatGPT salvas são consultadas em ciclos de cinco minutos. Um reset disponível com ID novo gera um aviso com a conta e a data de vencimento. Outros avisos são enviados quando restam até 24 horas e até 1 hora. O primeiro inventário de cada conta estabelece a referência; resets que já existiam não são anunciados como recém-recebidos, mas continuam elegíveis aos avisos de vencimento. IDs permitem detectar uma concessão mesmo quando outro reset foi consumido e o total permaneceu igual.

Avisos aceitos pelo macOS são registrados para evitar repetição após reiniciar. Se o Mac voltar a funcionar faltando menos de uma hora, envia apenas o aviso de uma hora, sem repetir o de 24 horas. Resets expirados, consumidos ou em resgate não geram avisos. Uma data desconhecida não impede a chegada de ser anunciada, mas não gera um prazo inventado. A consulta precisa de internet e o Mac precisa estar ligado e acordado; após dormir ou ficar offline, a checagem retoma. Foco e permissões do macOS controlam a apresentação dos banners.

## Compilação e instalação

Pré-requisitos: macOS 14+, Xcode Command Line Tools, Rust, Node e dependências do CC Switch instaladas com o pnpm da versão indicada no repositório. Execute `extras/reset-alerts/build.sh /caminho/absoluto/para/outputs`; o script constrói o emissor nativo com os metadados versionados, testa a política, constrói o frontend e o backend de produção e verifica as assinaturas locais dos aplicativos. Usa a instalação original em `/Applications/CC Switch.app` como base dos ícones e metadados do pacote. Execute a partir do código completo correspondente ao patch.

A instalação é feita no Mac de destino. Preserve uma cópia do aplicativo original, encerre o CC Switch normalmente e copie os dois aplicativos para `/Applications`. Depois execute `Ativar alertas.command`, mantendo `resolve-data-dir.sh` e `configuration.sh` junto dele. A ativação não substitui aplicativos nem encerra processos: exige que o CC Switch já esteja fechado, resolve o diretório de dados configurado no aplicativo, prepara os logs, grava a configuração do monitor e registra o início no login. O pacote não inclui contas ou credenciais. Consulte `LEIA-ME.txt` para os passos e a reversão.

No diretório de dados ativo, crie `reset-alerts.json` com `{"enabled":true,"notifier_path":"/caminho/absoluto/CC Switch Reset Alerts.app/Contents/MacOS/ResetAlerts"}` e permissão 0600. Abra o auxiliar uma vez para autorizar notificações. Se a permissão estiver negada, ative-a em Ajustes do Sistema → Notificações → CC Switch Reset Alerts. Nenhuma permissão é concedida automaticamente.

A configuração `~/.cc-switch/reset-alerts.json` contém apenas a ativação e o caminho do emissor. `reset-alerts-ledger.json` guarda a referência inicial e os recibos de notificações. `reset-alerts-status.json` mostra a última consulta, o resultado por conta e a autorização das notificações. Esses arquivos são privados do usuário. Não apague o ledger para resolver uma falha temporária: isso perderia os recibos.

## Atualizações e reversão

Esta é uma compilação local, não uma versão oficial publicada pelo projeto CC Switch. Uma atualização oficial pode substituir o backend e remover o monitor; o patch precisa ser reaplicado e validado na nova versão. O emissor possui assinatura ad hoc local.

Para reverter, encerre o CC Switch normalmente e aguarde sua saída; então descarregue somente o LaunchAgent `local.henrique.ccswitch-reset-alerts` e restaure `CC Switch original.app` como `/Applications/CC Switch.app`. Remova somente o LaunchAgent e desative `reset-alerts.json` (`enabled: false`). O histórico dos alertas pode ser preservado. Não altere o arquivo de contas OAuth. A cópia original fica na pasta de entregáveis usada na instalação.

## Validação e revisão

Os testes da política exercitam concessão sem mudança do saldo total, referência inicial, independência de contas, avisos nos dois limiares, reinício, retorno tardio, falha de entrega e inventários inválidos. A revisão usa `codex review` sobre o diff. A consulta real de todas as contas e a autorização do emissor são verificadas separadamente na instalação; testes sintéticos não provam que um banner foi mostrado na tela.
