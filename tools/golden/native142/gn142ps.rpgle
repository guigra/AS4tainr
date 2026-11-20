**FREE
// Stage 2, S2.142: calls of procedures with their own *PSSR: a division
// by zero in the procedure goes to its *PSSR, whose RETURN ends the call
// with a value (-1, and %STATUS in it); a call with no error returns as
// usual. Captured on PUB400 (tools/golden/native142.txt n142_pssr) for the
// native unit NPSSRPR.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

line = %char(safediv(10: 2)) + ' ' + %char(safediv(1: 0))
     + ' ' + %char(divstat(1: 0)) + ' ' + %char(divstat(9: 3));
say(line);
*inlr = *on;

dcl-proc safediv;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  dcl-s r int(10);
  r = a / b;
  return r;
  begsr *pssr;
    return -1;
  endsr;
end-proc;

dcl-proc divstat;
  dcl-pi *n int(10);
    a int(10) value;
    b int(10) value;
  end-pi;
  dcl-s r int(10);
  r = a / b;
  return r;
  begsr *pssr;
    return %status;
  endsr;
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
