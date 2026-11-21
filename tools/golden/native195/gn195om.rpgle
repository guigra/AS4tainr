**FREE
// Stage 2, S2.195: *OMIT for a parameter of a procedure of the module:
// %PARMS counts it, and its address is *NULL; with *NOPASS, not passed,
// passed as *OMIT and passed with a value. Captured on PUB400
// (tools/golden/native195.txt n195_omit) for the native unit NOMITLC.
ctl-opt dftactgrp(*no);
dcl-s line varchar(200);

line = f(1) + ' ' + f(1 : *omit) + ' ' + f(1 : 'x');
say(line);
*inlr = *on;

dcl-proc f;
  dcl-pi *n varchar(20);
    a int(10) const;
    b char(10) const options(*nopass : *omit);
  end-pi;
  dcl-s r varchar(20);
  r = %char(%parms());
  if %parms() >= 2;
    if %addr(b) = *null;
      r += 'N';
    else;
      r += %trim(b);
    endif;
  endif;
  return r;
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
