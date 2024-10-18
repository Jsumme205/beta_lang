use crate::betac_lexer::ast_types::context::ContextKind;
use crate::betac_lexer::ast_types::{defun::Argument, Expr};
use crate::betac_util::{session::BuildFxHasher, sso::OwnedYarn};
use crate::betac_util::{OptionExt, Yarn};
use crate::yarn;
use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, Write},
};

#[derive(Debug)]
pub enum BackendError {
    Other,
    Io(io::Error),
    None,
}

impl From<io::Error> for BackendError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl<T> TryFrom<Option<T>> for BackendError {
    type Error = T;

    fn try_from(value: Option<T>) -> Result<Self, Self::Error> {
        match value {
            Some(v) => Err(v),
            None => Ok(Self::None),
        }
    }
}

pub trait WriteIr: Debug {
    fn lower(self, writer: &mut IrCodegen) -> Result<(), BackendError>;
}

pub struct IrCodegen {
    buf: Vec<u8>,
    count: usize,
    id_vars: HashMap<OwnedYarn, usize, BuildFxHasher>,
}

impl IrCodegen {
    pub fn inc_and_return(&mut self) -> usize {
        self.count += 1;
        self.count
    }

    pub fn init() -> Self {
        Self {
            buf: Vec::new(),
            count: 0,
            id_vars: HashMap::default(),
        }
    }

    fn id_for_ident(&self, ident: &Yarn<'_>) -> Option<usize> {
        self.id_vars.get(&ident).map(|opt| *opt)
    }

    fn id_and_ident(&mut self, ident: OwnedYarn, id: usize) {
        self.id_vars.insert(ident, id);
    }

    pub fn as_str(&self) -> &str {
        unsafe { std::str::from_utf8_unchecked(&self.buf) }
    }

    pub fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.write(s.as_bytes())
    }
}

impl io::Write for IrCodegen {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        io::Write::write(&mut self.buf, buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::Write::flush(&mut self.buf)
    }
}

impl<'src> WriteIr for Argument<'src> {
    fn lower(self, writer: &mut IrCodegen) -> Result<(), BackendError> {
        let (ident, ty) = self;
        writer.write(ty.to_llvm_name().as_bytes())?;
        let id = writer.inc_and_return();
        let name = yarn!("%{}{}", ident, id);
        writer.id_and_ident(ident.leak(), id);
        writer.write(name.as_bytes())?;
        Ok(())
    }
}

pub fn write_args<'a>(args: Vec<Yarn<'a>>, writer: &mut IrCodegen) -> Result<(), BackendError> {
    writer.write_str("(")?;
    let len = args.len();
    for (i, arg) in args.into_iter().enumerate() {
        let id = writer.id_for_ident(&arg).unwrap();
        let arg = yarn!("%{arg}{id}");
        writer.write(arg.as_bytes())?;
        if i != (len - 1) {
            writer.write_str(", ")?;
        }
    }
    writer.write_str(")")?;
    Ok(())
}

impl<'src> WriteIr for Vec<Argument<'src>> {
    fn lower(self, writer: &mut IrCodegen) -> Result<(), BackendError> {
        writer.write("(".as_bytes())?;
        let len = self.len();
        for (i, arg) in self.into_iter().enumerate() {
            arg.lower(writer)?;
            if len - 1 != i {
                writer.write(", ".as_bytes())?;
            }
        }
        writer.write(")".as_bytes())?;
        Ok(())
    }
}

impl<'src> WriteIr for Expr<'src> {
    fn lower(
        self,
        writer: &mut crate::betac_backend::IrCodegen,
    ) -> Result<(), crate::betac_backend::BackendError> {
        match self {
            Self::Assignment {
                ident,
                ty,
                value,
                meta: _meta,
            } => {
                println!("ident_115: {ident}");
                let id = writer.inc_and_return();
                let name = yarn!("%{}{}", ident, id);
                writer.id_and_ident(ident.leak(), id);
                // since this uses a `Vec<u8>` internally, we really don't have to handle errors
                // but rust complains if we don't so we just unwrap
                writer.write(name.as_bytes()).unwrap();
                writer.write(&[b' ', b'=', b' ']).unwrap();
                if !matches!(
                    &*value,
                    Self::Call { ret_ty, .. }
                        | Self::Binary { ty: ret_ty, .. }
                    if ret_ty.can_be_implicitly_converted(ty)
                    || ty.can_be_implicitly_converted(*ret_ty)

                ) {
                    writer.write(ty.to_llvm_name().as_bytes()).unwrap();
                }
                match Box::into_inner(value) {
                    Self::Literal(lit) => {
                        writer.write(lit.as_bytes()).unwrap();
                    }
                    Self::Copy(ident) => {
                        println!("ident_130: {ident}");
                        let id = writer.id_for_ident(&ident).unwrap();
                        let ident = yarn!("%{ident}{id}");
                        writer.write(ident.as_bytes())?;
                    }
                    Self::Call {
                        ident,
                        args,
                        ret_ty,
                    } => {
                        println!("ident_165: {ident}");
                        println!("idents: {:#?}", writer.id_vars);
                        println!("args_167: {:#?}", args);
                        let id = writer.id_for_ident(&ident).unwrap();
                        let ident = yarn!("call {}@{ident}{id}", ret_ty.to_llvm_name());
                        writer.write(ident.as_bytes())?;
                        write_args(args, writer)?;
                    }
                    Self::Binary {
                        lhs, op, rhs, ty, ..
                    } => {
                        op.lower(writer)?;
                        writer.write_str(ty.to_llvm_name())?;
                        lhs.lower(writer)?;
                        writer.write_str(", ")?;
                        rhs.lower(writer)?;
                    }
                    _ => todo!(),
                }
                writer.write(&[b'\n']).unwrap();
            }
            Self::Defun {
                meta: _meta,
                args,
                expressions,
                return_ty,
                ident,
            } => {
                let ty = return_ty.to_llvm_name();
                let id = writer.inc_and_return();
                println!("id: {id}");
                let function_name = yarn!("define {ty}@{ident}{id}");
                writer.id_and_ident(ident.leak(), id);
                writer.write(function_name.as_bytes())?;
                args.lower(writer)?;
                writer.write("{\n".as_bytes())?;
                for expr in expressions {
                    expr.lower(writer)?;
                }
                writer.write("}".as_bytes())?;
            }
            Self::Eof => {
                writer.write(&[b'\0'])?;
            }
            Self::LitOrIdent(ident, _) => {
                let op = writer
                    .id_for_ident(&ident)
                    .try_catch(|id| yarn!("%{}{}", ident, id), || yarn!("{ident}"));
                writer.write(op.as_bytes())?;
            }
            Self::Literal(lit) => {
                writer.write_str(lit.as_str())?;
            }

            Self::Binary {
                lhs,
                op,
                rhs,
                ty,
                context_kind,
            } => match context_kind {
                ContextKind::Block => {
                    todo!()
                }
                ContextKind::Function => {
                    writer.write_str("ret ")?;
                    op.lower(writer)?;
                    writer.write_str(ty.to_llvm_name())?;
                    lhs.lower(writer)?;
                    writer.write_str(", ")?;
                    rhs.lower(writer)?;
                    writer.write_str("\n")?;
                }
                _ => return Err(BackendError::Other),
            },

            expr => {
                println!("expr_140: {expr:#?}");
                todo!()
            }
        }
        Ok(())
    }
}
