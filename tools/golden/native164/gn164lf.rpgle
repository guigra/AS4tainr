**FREE
// Stage 2, S2.164: file operations in the main procedure on a LIKEFILE
// file with EXTFILE while a procedure has a LIKEFILE parameter: OPEN,
// the file passed to the procedure (which reads it), SETLL *HIVAL, READP
// into a data structure, CLOSE. Captured on PUB400
// (tools/golden/native164.txt n164_likef) for the native unit NLIKEF.
// GN164F holds the rows 1 'aa', 2 'bb', 3 'cc'.
ctl-opt dftactgrp(*no);
dcl-f gn164f template rename(gn164f : gn164r) block(*no);
dcl-f t1 likefile(gn164f) usropn extfile(fname);
dcl-ds rec likerec(gn164r);
dcl-s fname char(21) inz('GN164F');
dcl-s n int(10);
dcl-s line varchar(80);

open t1;
n = countAll(t1);
line = %char(n);
setll *hival t1;
readp t1 rec;
line += ' ' + %char(rec.n) + rec.s;
close t1;
say(line);
*inlr = *on;

dcl-proc countAll;
  dcl-pi *n int(10);
    f likefile(gn164f);
  end-pi;
  dcl-ds r likerec(gn164r);
  dcl-s c int(10);
  setll *start f;
  read f r;
  dow not %eof(f);
    c += 1;
    read f r;
  enddo;
  return c;
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
