**FREE
// Stage 2, S2.60: whole data structures and hexadecimal literals as
// arguments of program calls. The user space APIs with the error code
// structure (a data structure passed by reference), a receiver data
// structure for QUSRTVUS (OPTIONS(*VARSIZE)), and x'C1C2' as a CONST
// character argument. Captured on PUB400 (tools/golden/native60.txt
// n60_api) for the native unit NCALLDS.
ctl-opt dftactgrp(*no);
dcl-c SPACE 'GN60US    GUIGRA261';
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
say('CRT AVAIL ' + %char(avail));
chgus(SPACE : 1 : 4 : 'ABCD' : '0' : errc);
chgus(SPACE : 5 : 2 : x'C1C2' : '0' : errc);
say('CHG AVAIL ' + %char(avail));
rtvus(SPACE : 1 : 8 : rcv : errc);
say('RTV [' + rcv + '] ' + %char(avail));
say('FIRST [' + first + '] SECOND [' + second + ']');
dltus(SPACE : errc);
say('DLT AVAIL ' + %char(avail));
rtvus(SPACE : 1 : 8 : rcv : errc);
say('GONE AVAIL ' + %char(avail > 0) + ' ' + msgid);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
