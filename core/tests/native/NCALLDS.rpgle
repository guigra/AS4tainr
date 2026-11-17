**FREE
// Native unit NCALLDS (S2.60): whole data structures and hexadecimal
// literals as arguments of program calls. The user space APIs with the
// error code structure (a data structure passed by reference), a
// receiver data structure for QUSRTVUS (OPTIONS(*VARSIZE)), and x'C1C2'
// as a CONST character argument. The program captured on PUB400, with
// the user space in QGPL and DSPLY for its output.
// Source: golden capture tools/golden/native60 n60_api (PUB400, IBM i
// 7.5, 2026-11-17).
ctl-opt dftactgrp(*no);
dcl-c SPACE 'NC60US    QGPL';
dcl-pr crtus extpgm('QUSCRTUS');
  name char(20) const;
  attr char(10) const;
  size int(10) const;
  init char(1) const;
  aut char(10) const;
  text char(50) const;
  repl char(10) const;
  err char(32767) options(*varsize);
end-pr;
dcl-pr chgus extpgm('QUSCHGUS');
  name char(20) const;
  start int(10) const;
  len int(10) const;
  data char(32767) const options(*varsize);
  frc char(1) const;
  err char(32767) options(*varsize);
end-pr;
dcl-pr rtvus extpgm('QUSRTVUS');
  name char(20) const;
  start int(10) const;
  len int(10) const;
  rcv char(32767) options(*varsize);
  err char(32767) options(*varsize);
end-pr;
dcl-pr dltus extpgm('QUSDLTUS');
  name char(20) const;
  err char(32767) options(*varsize);
end-pr;
dcl-ds errc;
  provided int(10) inz(64);
  avail int(10);
  msgid char(7);
  rsv char(1);
  msgdta char(48);
end-ds;
dcl-ds rcv;
  first char(4);
  second char(4);
end-ds;
crtus(SPACE : 'TEST' : 64 : x'40' : '*ALL' : 'S2.60' : '*YES' : errc);
dsply ('CRT AVAIL ' + %char(avail));
chgus(SPACE : 1 : 4 : 'ABCD' : '0' : errc);
chgus(SPACE : 5 : 2 : x'C1C2' : '0' : errc);
dsply ('CHG AVAIL ' + %char(avail));
rtvus(SPACE : 1 : 8 : rcv : errc);
dsply ('RTV [' + rcv + '] ' + %char(avail));
dsply ('FIRST [' + first + '] SECOND [' + second + ']');
dltus(SPACE : errc);
dsply ('DLT AVAIL ' + %char(avail));
rtvus(SPACE : 1 : 8 : rcv : errc);
dsply ('GONE AVAIL ' + %char(avail > 0) + ' ' + msgid);
*inlr = *on;
