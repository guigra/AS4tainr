**FREE
// Stage 2, S2.173: CALLP(E) of a procedure that fails (a division by
// zero) and of one that does not, %ERROR and %STATUS after each.
// Captured on PUB400 (tools/golden/native173.txt n173_callpe) for the
// native unit NCALLPE.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);
dcl-s r int(10);

callp(e) boom(0);
line = %char(%error) + ' ' + %char(%status) + ' ' + %char(r);
callp(e) boom(2);
line += ' ' + %char(%error) + ' ' + %char(%status) + ' ' + %char(r);
say(line);
*inlr = *on;

dcl-proc boom;
  dcl-pi *n;
    d int(10) const;
  end-pi;
  r = 10 / d;
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
