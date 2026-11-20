**FREE
// Stage 2, S2.172: WRITE(E) of a database file: a new key, then a key
// that is there (a duplicate key on the primary key), %ERROR and %STATUS
// after each. GN172F has the primary key K and holds the row 1 'aa'; it
// is opened in arrival sequence. Captured on PUB400
// (tools/golden/native172.txt n172_writee) for the native unit NWRITEE.
ctl-opt dftactgrp(*no);
dcl-f gn172f usage(*output) rename(gn172f : gn172r);
dcl-s line varchar(80);

k = 2;
s = 'bb';
write(e) gn172r;
line = %char(%error) + ' ' + %char(%status(gn172f));
k = 1;
s = 'zz';
write(e) gn172r;
line += ' ' + %char(%error) + ' ' + %char(%status(gn172f));
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
