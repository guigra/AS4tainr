**FREE
// Stage 2, S2.192: UNLOCK of data areas: IN *LOCK, then
// UNLOCK, then OUT(E) of the field (its %ERROR and %STATUS), and IN again.
// Captured on PUB400 (tools/golden/native192.txt n192_unlock) for the
// native unit NDTAUNL.
ctl-opt dftactgrp(*no);
dcl-pr qcmdexc extpgm('QCMDEXC');
  cmd char(80) const;
  len packed(15:5) const;
end-pr;
dcl-s val char(10) dtaara('QTEMP/GN192A');
dcl-s oth char(10) dtaara('QTEMP/GN192B');
dcl-s line varchar(200);

monitor;
  qcmdexc('DLTDTAARA DTAARA(QTEMP/GN192A)' : 80);
on-error;
endmon;
monitor;
  qcmdexc('DLTDTAARA DTAARA(QTEMP/GN192B)' : 80);
on-error;
endmon;
qcmdexc('CRTDTAARA DTAARA(QTEMP/GN192A) TYPE(*CHAR) LEN(10) VALUE(''one'')' : 80);
qcmdexc('CRTDTAARA DTAARA(QTEMP/GN192B) TYPE(*CHAR) LEN(10) VALUE(''two'')' : 80);
in *lock val;
line = %trim(val);
unlock val;
val = 'changed';
out(e) val;
line += ' ' + %char(%error) + ' ' + %char(%status);
in *lock val;
in *lock oth;
line += ' ' + %trim(val) + ' ' + %trim(oth);
// One data area by name: as4tainr's free-form parser takes *DTAARA as
// the name DTAARA (fidelity.md, interpreter defects).
unlock oth;
oth = 'other';
out(e) oth;
line += ' ' + %char(%error) + ' ' + %char(%status);
in oth;
line += ' ' + %trim(oth);
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
