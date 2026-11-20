**FREE
// Stage 2, S2.151: *ON and *OFF for an indicator parameter passed by
// value, in a DSPLY'd call, an assignment and a condition. Captured on
// PUB400 (tools/golden/native151.txt n151_figind) for the native unit
// NINDFGA.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

line = flag(*on) + flag(*off);
if flag(*off) = 'no';
  line += ' if';
endif;
line += ' ' + %char(count(*on) + count(*off) * 10);
say(line);
*inlr = *on;

dcl-proc flag;
  dcl-pi *n varchar(10);
    f ind value;
  end-pi;
  if f;
    return 'yes';
  endif;
  return 'no';
end-proc;

dcl-proc count;
  dcl-pi *n int(10);
    f ind value;
  end-pi;
  if f = *on;
    return 1;
  endif;
  return 2;
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
