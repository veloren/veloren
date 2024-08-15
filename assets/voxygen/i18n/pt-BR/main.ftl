main-username = Nome de Usuário
main-server = Servidor
main-password = Senha
main-connecting = Conectando
main-creating_world = Criando Mundo
main-tip = Dica:
main-unbound_key_tip = não vinculado
main-notice =
    Bem-vindo a versão alfa de Veloren!

    Antes de iniciar a diversão, tenha algumas coisinhas em mente:

    - Esta é uma versão muito experimental. Prepare-se para defeitos(bugs), jogabilidade inacabada, mecânicas desbalanceadas ou faltantes e funcionalidades ausentes.

    - Caso você possua comentários construtivos para tecer ou defeitos a serem reportados, você pode nos contactar através do repositório no Gitlab ou servidores Discord e Matrix.

    - Veloren é open source. Você é livre pra jogar, modificar e redistribuir desde que em acordo com a versão 3 da GNU General Public license.

    - Veloren é um projeto comunitário sem fins lucrativos, e todos que trabalham nele são voluntários.
    Se você gosta do que viu, sinta-se a vontade para ingressar nos nossos grupos de trabalho!

    Obrigado pelo seu tempo gasto lendo este aviso, esperamos que você goste do jogo!

    ~ Equipe de desenvolvimento
main-login_process =
    Sobre o modo multijogador:

    Por favor, note que você precisa de uma conta para jogar em servidores com autenticação ativada.

    Você pode criar uma conta em:
    https://veloren.net/account/
main-login-server_not_found = Servidor não encontrado.
main-login-authentication_error = Erro de autenticação no servidor.
main-login-internal_error = Erro interno no cliente. Dica: Provavelmente o personagem do jogador foi deletado.
main-login-failed_auth_server_url_invalid = Falha na conexão com o servidor de autenticação.
main-login-insecure_auth_scheme = A autenticação através do esquema HTTP não é suportada. É inseguro! A propósito de desenvolvimento, o HTTP é permitido no 'localhost' ou em builds no modo debug.
main-login-server_full = Servidor lotado.
main-login-untrusted_auth_server = Servidor de autenticação não confiado.
main-login-timeout = Tempo esgotado: Servidor não respondeu a tempo. Dica: o servidor pode estar sobrecarregado ou há problemas de rede.
main-login-server_shut_down = Servidor encerrou ou desligou.
main-login-network_error = Erro de Rede.
main-login-network_wrong_version = O servidor está executando uma versão diferente da sua. Dica: verifique se há atualizações no seu cliente do jogo.
main-login-failed_sending_request = Requisição ao servidor de autenticação falhou.
main-login-invalid_character = O personagem selecionado é inválido.
main-login-client_crashed = Cliente abortou.
main-login-not_on_whitelist = Você não está na lista dos membros permitidos(whitelist) do servidor que está tentando ingressar.
main-login-banned = Você foi banido pelo seguinte motivo:
main-login-kicked = Você foi expulso pelo seguinte motivo:
main-login-select_language = Escolha um Idioma
main-login-client_version = Versão do cliente
main-login-server_version = Versão do servidor
main-login-client_init_failed = Cliente falhou ao inicializar: { $init_fail_reason }
main-login-username_bad_characters = Nome de usuário contém caracteres inválidos! (Apenas alfanuméricos, '_' e '-' são perimtidos).
main-login-username_too_long = Nome de usuário muito longo! Tamanho máximo: { $max_len }
main-servers-select_server = Escolha um servidor
main-servers-singleplayer_error = Falha ao conetar no servidor interno: { $sp_error }
main-servers-network_error = Erro de socket/rede: { $raw_error }
main-servers-participant_error = Erro de rede/protocolo do participante: { $raw_error }
main-servers-stream_error = Erro de conexão/compressão/(de)serialização do cliente: { $raw_error }
main-servers-database_error = Erro de Base da dados do Servidor: { $raw_error }
main-servers-persistence_error = Erro de persistência do servidor (Possivelmente relacionado a Asset/Personagens): { $raw_error }
main-servers-other_error = Erro geral de servidor: { $raw_error }
main-credits = Créditos
main-credits-created_by = criado por
main-credits-music = Música
main-credits-fonts = Fontes
main-credits-other_art = Outras Artes
main-credits-contributors = Contribuidores
loading-tips =
    .a0 = Pressione '{ $gameinput-togglelantern }' para acender sua lâmpada.
    .a1 = Pressione '{ $gameinput-help }' para visualizar suas teclas de atalho.
    .a2 = Você pode digitar /say ou /s para conversar apenas com jogadores próximos a você.
    .a3 = Você pode digitar /region ou /r para conversar apenas com jogadores a poucas centenas de blocos de você.
    .a4 = Admins podem usar o comando /build para entrar no modo construção.
    .a5 = Você pode digitar /group ou /g para conversar apenas com jogadores do seu grupo.
    .a6 = Para enviar mensagens privadas digite /tell seguido do nome do jogador desejado.
    .a7 = Busque sempre comida, baús e outros espólios espalhados pelo mundo!
    .a8 = Inventário cheio de comidas? Tente criar alimentos melhores com elas!
    .a9 = Imaginando o que há pra fazer? Cavernas estão marcadas com pontos marrons no mapa!
    .a10 = Não esqueça de ajustar as configurações gráficas. Pressione '{ $gameinput-settings }' para abrir as configurações.
    .a11 = Jogar com outros é divertido! Pressione '{ $gameinput-social }' para ver quem está online.
    .a12 = Pressione '{ $gameinput-dance }' para dançar. Hora da festa!
    .a13 = Pressione '{ $gameinput-glide }' para abrir o Planador e conquistar os céus.
    .a14 = Veloren ainda está no Pre-Alpha. Estamos nos empenhando ao máximo para melhorar a cada dia!
    .a15 = Se quiser ingressar no time de Desenvolvedores ou apenas conversar conosco, acesse o nosso servidor do Discord.
    .a16 = Você pode exibir sua saúde em sua barra de vida nas opções.
    .a17 = Sente ao redor de uma fogueira (usando a tecla '{ $gameinput-sit }') para lentamente se recuperar de lesões.
    .a18 = Precisa de uma mochila maior para sua jornada? Pressione '{ $gameinput-crafting }' para abrir o menu de criação!
    .a19 = Pressoine '{ $gameinput-roll }' para rolar. Rolamentos podem ser usados para se movimentar mais rapidamente e desviar de ataques dos inimigos.
    .a20 = Se perguntando para que este item serve? Busque por 'input:<item name>' na área de criação para ver receitas que utilizam tal item.
    .a21 = Você pode capturar a tela pressionando '{ $gameinput-screenshot }'.
main-singleplayer-delete = Deletar
main-singleplayer-random_seed = Aleatório
main-singleplayer-world_name = Nome do mundo
main-singleplayer-map_scale = Escala vertical
main-singleplayer-map_erosion_quality = Qualidade da erosão
main-singleplayer-map_shape = Formato
main-singleplayer-play = Jogar
main-singleplayer-generate_and_play = Gerar & Jogar
main-server-rules = Este servidor possui regras que precisam ser aceitas.
main-server-rules-seen-before = As regras deste servidor mudaram deste a última vez em que você as aceitou.
main-singleplayer-map_large_warning = Aviso: Mundos grandes levarão mais tempo durante a primeira inicialização.
menu-singleplayer-confirm_regenerate = Tem certeza que deseja regerar "{ $world_name }"?
main-singleplayer-new = Novo
main-singleplayer-regenerate = Regerar
main-singleplayer-create_custom = Criar Personalizado
main-singleplayer-seed = Semente
menu-singleplayer-confirm_delete = Tem certeza que deseja deletar "{ $world_name }"?
main-singleplayer-day_length = Duração do dia
main-singleplayer-size_lg = Tamanho logarítmico
