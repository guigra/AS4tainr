**FREE
// Stage 2, S2.116: OPTIONS(*TRIM) on parameters of local procedures: a
// literal with blanks on both sides to a VALUE varying parameter, a
// field with blanks to a CONST character parameter, and a %TRIM
// argument. Captured on PUB400 (tools/golden/native116.txt n116_trim)
// for the native unit NOPTTRM.
ctl-opt dftactgrp(*no);
dcl-s fld char(10) inz('  xy z');
dcl-s line varchar(200);
line = show('  ab  ') + ' ' + showc(fld) + ' ' + show(%trimr(fld));
say(line);
*inlr = *on;

dcl-proc show;
  dcl-pi *n varchar(30);
    p varchar(20) value options(*trim);
  end-pi;
  return '[' + p + ']' + %char(%len(p));
end-proc;

dcl-proc showc;
  dcl-pi *n varchar(30);
    p char(8) const options(*trim);
  end-pi;
  return '[' + p + ']';
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
