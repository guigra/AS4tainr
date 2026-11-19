**FREE
// Stage 2, S2.105: indicator fields compared with *ON and *OFF (on
// either side), with another indicator field (= and <>), in IF and DOW,
// and in a procedure with its own. Captured on PUB400
// (tools/golden/native105.txt n105_indcmp) for the native unit NINDCMP.
ctl-opt dftactgrp(*no);
dcl-s flag ind inz(*on);
dcl-s spare ind;
dcl-s k int(5);
dcl-s line varchar(80);
if flag = *on;
  line += 'a';
endif;
if flag = *off;
  line += 'X';
endif;
if *off = spare;
  line += 'b';
endif;
if flag = spare;
  line += 'X';
endif;
if flag <> spare;
  line += 'c';
endif;
spare = *on;
if flag = spare;
  line += 'd';
endif;
dow spare = *on;
  k += 1;
  if k = 3;
    spare = *off;
  endif;
enddo;
line += %char(k) + ' ' + local();
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(10);
  end-pi;
  dcl-s mine ind inz(*off);
  if mine <> *on;
    return 'ok';
  endif;
  return 'no';
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
