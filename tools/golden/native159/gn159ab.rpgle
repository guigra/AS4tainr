**FREE
// Stage 2, S2.159: %ADDR of a subfield of a based data structure: the
// basing pointer's place plus the subfield's offset, written through by
// another based field, compared with the pointer moved on, and again
// after the basing pointer moves. Captured on PUB400
// (tools/golden/native159.txt n159_addrbased) for the native unit NADDRBS.
ctl-opt dftactgrp(*no);
dcl-s buf char(40) inz(*all'.');
dcl-ds ent based(p);
  name char(10);
  data char(10);
end-ds;
dcl-s v char(10) based(q);
dcl-s p pointer;
dcl-s q pointer;
dcl-s line varchar(80);

p = %addr(buf);
q = %addr(data);
v = 'hello';
if q = p + 10;
  line = 'same';
endif;
p = %addr(buf) + 20;
q = %addr(data);
v = 'world';
line += ' ' + buf;
say(line);
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
