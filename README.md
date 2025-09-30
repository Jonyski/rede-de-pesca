# rede-de-pesca

Este é um projeto feito para a matéria de Redes de Computadores


## Tarefas

- [ ] Envio de todos os tipos de mensagens pela rede (server/mod.rs)
- [ ] Tratar a chegada de todos os tipos de mensagens (lib.rs)
- [ ] Implementar sistema de nomes de usuário
- [ ] Criar invetario global de peixeis
- [ ] Implementar sistema de trocas
- [ ] Implementar uma interface de terminal (tui/mod.rs)
- [ ] Implementar todos os comandos da interface ($i, $t, etc) (tui/mod.rs)

## Mecânicas

Os usuários da Rede de Pesca podem:

- Enviar mensagens uns para os outros e para todos os usuários (_broadcast_)
- Pescar uma grande variedade de peixes com diferentes raridades
- Trocar peixes com outros usuários

## Tipos de mensagem

- Envio de mensagem 1:1
- Inspecionar peixes
- Inventário
- Broadcast 1:N
- Pedido de troca de peixe
- Resposta de pedido de troca de peixe

## Protocolo

Tipo: Mensagem;

Remetente: @pedrinho

Destinatario: @joao

Texto: "aaaaaaaaaa";

-----------------------------

Tipo: Inspeçao;

Nome: "carinha";

-----------------------------

Tipo: Inventário;

Inventário: peixe1|12,peixes2|13;

-----------------------------

Tipo: Broadcast;

Remetente: @jao

Texto: "pessoa tal achou um peixe raro";

-----------------------------

Tipo: Pedido de Troca;

Proposta: tupiniqui|12 > lambari|1;

-----------------------------

Tipo: Confirma Troca;

Proposta: Sim/Não;

## Comandos

-> (broadcast)

-> @joao aaaaaaaaa

-> :[p]escar

-> :[i]nventario

-> :[i]nventario @jao

-> :[t]roca @pedrinho n peixe > m peixe2

-> :[c]onfirmar [s]im/[n]ao
