**FREE
// Stage 2, S2.126: indicators compared with *ON and *OFF in IF and DOU.
// Captured on PUB400 (tools/golden/native126.txt n126_indfig) for the
// native unit NINDFIG.
ctl-opt dftactgrp(*no);
dcl-s n int(10);
dcl-s line varchar(80);

*in50 = *on;
*in51 = *off;
if *in50 = *on;
  line += 'a';
endif;
if *in51 = *on;
  line += 'b';
endif;
if *in51 = *off;
  line += 'c';
endif;
if *off <> *in50;
  line += 'd';
endif;
if *in50 <> *off;
  line += 'e';
endif;
dou *in99 = *on;
  n += 1;
  if n = 3;
    *in99 = *on;
  endif;
enddo;
line += %char(n);
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
