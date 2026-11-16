**FREE
// Stage 2, S2.51: pointer parameters (by value, by reference, *NULL) and
// OPTIONS(*VARSIZE) (QCMDEXC with a command field shorter than its
// parameter). Captured on PUB400 (tools/golden/native52.txt n52_parm) for
// the native unit NPARM.
ctl-opt dftactgrp(*no);
dcl-pr qcmdexc extpgm('QCMDEXC');
  cmd char(3000) options(*varsize) const;
  len packed(15:5) const;
end-pr;
dcl-ds lda dtaara(*lda) len(20);
  ldatext char(20);
end-ds;
dcl-s buf char(6) inz('ABCDEF');
dcl-s buf2 char(6) inz('XYZXYZ');
dcl-s p pointer;
dcl-s cmd char(60);
p = %addr(buf);
say('FIRST [' + first3(p) + ']');
say('ADDR [' + first3(%addr(buf2)) + ']');
say('MOVED [' + first3(%addr(buf) + 2) + ']');
point(p);
say('POINTED [' + first3(p) + ']');
say('NULL ' + isnull(*null));
say('SET ' + isnull(p));
cmd = 'CHGDTAARA DTAARA(*LDA (1 10)) VALUE(''VARSIZE OK'')';
qcmdexc(cmd : %len(%trimr(cmd)));
in lda;
say('LDA [' + %subst(ldatext : 1 : 10) + ']');
*inlr = *on;
dcl-proc first3;
  dcl-pi *n char(3);
    at pointer value;
  end-pi;
  dcl-s c char(3) based(at);
  return c;
end-proc;
dcl-proc point;
  dcl-pi *n;
    q pointer;
  end-pi;
  q = %addr(buf2);
end-proc;
dcl-proc isnull;
  dcl-pi *n char(3);
    at pointer value;
  end-pi;
  if at = *null;
    return 'YES';
  endif;
  return 'NO';
end-proc;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
