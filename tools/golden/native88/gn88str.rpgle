**FREE
// Stage 2, S2.88: %STR as a value. A field holding 'Hello', a null,
// 'World' and a null: %STR of its address (up to the first null), with a
// maximum shorter than that, of a pointer moved past the first null, of a
// pointer field, and its %LEN. Captured on PUB400
// (tools/golden/native88.txt n88_str) for the native unit NSTRPTR.
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
say(line + %char(%len(%str(p))));
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
