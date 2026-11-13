**FREE
// Changes of CONST parameters besides EVAL of the name, for the review
// of upstream PR #11: CLEAR, %SUBST as a target, a LIKEDS subfield, an
// array element.
ctl-opt dftactgrp(*no);
dcl-ds t qualified template;
  a char(1);
end-ds;
dcl-s s char(4) inz('abcd');
dcl-ds d likeds(t);
dcl-s arr int(10) dim(2);
f1(s);
f2(d);
f3(arr);
*inlr = *on;
dcl-proc f1;
  dcl-pi *n;
    p char(4) const;
  end-pi;
  clear p;
  %subst(p:1:1) = 'x';
end-proc;
dcl-proc f2;
  dcl-pi *n;
    p likeds(t) const;
  end-pi;
  p.a = 'x';
end-proc;
dcl-proc f3;
  dcl-pi *n;
    p int(10) dim(2) const;
  end-pi;
  p(1) = 3;
end-proc;
