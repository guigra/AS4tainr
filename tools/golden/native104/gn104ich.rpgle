**FREE
// Stage 2, S2.104: indicator fields as character data: %CHAR of one,
// and indicator fields in concatenations (with text, with *INxx, two
// together), in the main procedure and in a procedure with its own.
// Captured on PUB400 (tools/golden/native104.txt n104_indchar) for the
// native unit NINDCHR.
ctl-opt dftactgrp(*no);
dcl-s flag ind inz(*on);
dcl-s spare ind;
dcl-ds st qualified;
  done ind inz(*on);
end-ds;
dcl-s line varchar(80);
*in42 = *on;
line = %char(flag) + %char(spare) + ' ';
line += 'f=' + flag + ' s=' + spare + ' ' + flag + spare + *in42 + st.done;
line += ' ' + local();
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(20);
  end-pi;
  dcl-s mine ind;
  mine = *on;
  return '<' + mine + %char(mine) + '>';
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
