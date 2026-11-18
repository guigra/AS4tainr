**FREE
// Native unit NSTRPTR (S2.88): %STR as a value (of an address, with a
// maximum, of a moved pointer, of a pointer field, its %LEN). The program
// captured on PUB400, with DSPLY for its output.
// Source: golden capture tools/golden/native88 n88_str (PUB400, IBM i 7.5,
// 2026-11-18).
ctl-opt dftactgrp(*no);
dcl-s buf char(20);
dcl-s p pointer;
dcl-s line varchar(52);
buf = 'Hello' + x'00' + 'World' + x'00';
line = 'A ' + %str(%addr(buf)) + '|';
line += %str(%addr(buf): 3) + '|';
p = %addr(buf) + 6;
line += %str(p) + '|';
line += %str(%addr(buf) + 1: 10) + '|';
dsply (line + %char(%len(%str(p))));
*inlr = *on;
