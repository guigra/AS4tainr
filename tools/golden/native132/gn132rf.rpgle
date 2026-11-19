**FREE
// Stage 2, S2.132: RETURN of figurative constants: *ON and *OFF from an
// indicator procedure, *BLANKS, *ZEROS, *HIVAL and *ALL into the return
// types. Captured on PUB400 (tools/golden/native132.txt n132_retfig) for
// the native unit NRETFIG.
ctl-opt dftactgrp(*no);
dcl-s line varchar(80);

line = %char(isbig(7)) + %char(isbig(2)) + '[' + blank() + ']'
     + %char(zero()) + ' ' + %char(hi()) + ' ' + allx();
say(line);
*inlr = *on;

dcl-proc isbig;
  dcl-pi *n ind;
    n int(10) value;
  end-pi;
  if n > 5;
    return *on;
  endif;
  return *off;
end-proc;

dcl-proc blank;
  dcl-pi *n char(3);
  end-pi;
  return *blanks;
end-proc;

dcl-proc zero;
  dcl-pi *n packed(5:2);
  end-pi;
  return *zeros;
end-proc;

dcl-proc hi;
  dcl-pi *n packed(3:0);
  end-pi;
  return *hival;
end-proc;

dcl-proc allx;
  dcl-pi *n char(4);
  end-pi;
  return *all'ab';
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
