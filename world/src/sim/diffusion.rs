use super::Alt;
use rayon::prelude::*;

/// From https://github.com/fastscape-lem/fastscapelib-fortran/blob/master/src/Diffusion.f90
///
/// See https://fastscape-lem.github.io/fastscapelib-fortran
///
/// nx = grid size x
///
/// ny = grid size y
///
/// xl = x-dimension of the model topography in meters (double precision)
///
/// yl = y-dimension of the model topography in meters (double precision)
///
/// dt = length of the time step in years (double precision)
///
/// ibc = boundary conditions for grid.  For now, we assume all four boundaries are fixed height,
/// so this parameter is ignored.
///
/// h = heights of cells at each cell in the grid.
///
/// b = basement height at each cell in the grid (see https://en.wikipedia.org/wiki/Basement_(geology)).
///
/// kd = bedrock transport coefficient (or diffusivity) for hillslope processes in meter squared per year
/// (double precision) at each cell in the grid.
///
/// kdsed = sediment transport coefficient (or diffusivity) for hillslope processes in meter squared per year;
/// (double precision;) note that when kdsed < 0, its value is not used, i.e., kd for sediment and bedrock have
/// the same value, regardless of sediment thickness
/* subroutine Diffusion ()

  ! subroutine to solve the diffusion equation by ADI

  use FastScapeContext

  implicit none
*/
pub fn diffusion(
    nx: usize,
    ny: usize,
    xl: f64,
    yl: f64,
    dt: f64,
    _ibc: (),
    h: &mut [Alt],
    b: &mut [Alt],
    kd: impl Fn(usize) -> f64,
    kdsed: f64,
) {
    let aij = |i: usize, j: usize| j * nx + i;
    /*
      double precision, dimension(:), allocatable :: f,diag,sup,inf,res
      double precision, dimension(:,:), allocatable :: zint,kdint,zintp
      integer i,j,ij
      double precision factxp,factxm,factyp,factym,dx,dy
    */
    let mut f: Vec<f64>;
    let mut diag: Vec<f64>;
    let mut sup: Vec<f64>;
    let mut inf: Vec<f64>;
    let mut res: Vec<f64>;
    let mut zint: Vec<f64>;
    let mut kdint: Vec<f64>;
    let mut zintp: Vec<f64>;
    let mut ij: usize;
    let mut factxp: f64;
    let mut factxm: f64;
    let mut factyp: f64;
    let mut factym: f64;
    let dx: f64;
    let dy: f64;
    /*
      character cbc*4

      !print*,'Diffusion'

      write (cbc,'(i4)') ibc

      dx=xl/(nx-1)
      dy=yl/(ny-1)
    */
    // 2048*32/2048/2048
    // 1 / 64 m
    dx = xl / /*(nx - 1)*/nx as f64;
    dy = yl / /*(ny - 1)*/ny as f64;
    /*
      ! creates 2D internal arrays to store topo and kd

      allocate (zint(nx,ny),kdint(nx,ny),zintp(nx,ny))
    */
    zint = vec![Default::default(); nx * ny];
    kdint = vec![Default::default(); nx * ny];
    /*
      do j=1,ny
        do i=1,nx
          ij=(j-1)*nx+i
          zint(i,j)=h(ij)
          kdint(i,j)=kd(ij)
          if (kdsed.gt.0.d0 .and. (h(ij)-b(ij)).gt.1.d-6) kdint(i,j)=kdsed
        enddo
      enddo

      zintp = zint
    */
    for j in 0..ny {
        for i in 0..nx {
            // ij = vec2_as_uniform_idx(i, j);
            ij = aij(i, j);
            zint[ij] = h[ij] as f64;
            kdint[ij] = kd(ij);
            if kdsed > 0.0 && (h[ij] - b[ij]) > 1.0e-6 {
                kdint[ij] = kdsed;
            }
        }
    }

    zintp = zint.clone();
    /*
      ! first pass along the x-axis

      allocate (f(nx),diag(nx),sup(nx),inf(nx),res(nx))
      f=0.d0
      diag=0.d0
      sup=0.d0
      inf=0.d0
      res=0.d0
      do j=2,ny-1
    */
    f = vec![0.0; nx];
    diag = vec![0.0; nx];
    sup = vec![0.0; nx];
    inf = vec![0.0; nx];
    res = vec![0.0; nx];
    for j in 1..ny - 1 {
        /*
            do i=2,nx-1
            factxp=(kdint(i+1,j)+kdint(i,j))/2.d0*(dt/2.)/dx**2
            factxm=(kdint(i-1,j)+kdint(i,j))/2.d0*(dt/2.)/dx**2
              factyp=(kdint(i,j+1)+kdint(i,j))/2.d0*(dt/2.)/dy**2
            factym=(kdint(i,j-1)+kdint(i,j))/2.d0*(dt/2.)/dy**2
            diag(i)=1.d0+factxp+factxm
            sup(i)=-factxp
            inf(i)=-factxm
            f(i)=zintp(i,j)+factyp*zintp(i,j+1)-(factyp+factym)*zintp(i,j)+factym*zintp(i,j-1)
            enddo
        */
        for i in 1..nx - 1 {
            factxp = (kdint[aij(i + 1, j)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factxm = (kdint[aij(i - 1, j)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factyp = (kdint[aij(i, j + 1)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            factym = (kdint[aij(i, j - 1)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            diag[i] = 1.0 + factxp + factxm;
            sup[i] = -factxp;
            inf[i] = -factxm;
            f[i] = zintp[aij(i, j)] + factyp * zintp[aij(i, j + 1)]
                - (factyp + factym) * zintp[aij(i, j)]
                + factym * zintp[aij(i, j - 1)];
        }
        /*
        ! left bc
            if (cbc(4:4).eq.'1') then
            diag(1)=1.
            sup(1)=0.
            f(1)=zintp(1,j)
            else
            factxp=(kdint(2,j)+kdint(1,j))/2.d0*(dt/2.)/dx**2
            factyp=(kdint(1,j+1)+kdint(1,j))/2.d0*(dt/2.)/dy**2
            factym=(kdint(1,j-1)+kdint(1,j))/2.d0*(dt/2.)/dy**2
            diag(1)=1.d0+factxp
            sup(1)=-factxp
            f(1)=zintp(1,j)+factyp*zintp(1,j+1)-(factyp+factym)*zintp(1,j)+factym*zintp(1,j-1)
            endif
        */
        if true {
            diag[0] = 1.0;
            sup[0] = 0.0;
            f[0] = zintp[aij(0, j)];
        } else {
            // reflective boundary
            factxp = (kdint[aij(1, j)] + kdint[aij(0, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factyp = (kdint[aij(0, j + 1)] + kdint[aij(0, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            factym = (kdint[aij(0, j - 1)] + kdint[aij(0, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            diag[0] = 1.0 + factxp;
            sup[0] = -factxp;
            f[0] = zintp[aij(0, j)] + factyp * zintp[aij(0, j + 1)]
                - (factyp + factym) * zintp[aij(0, j)]
                + factym * zintp[aij(0, j - 1)];
        }
        /*
        ! right bc
            if (cbc(2:2).eq.'1') then
            diag(nx)=1.
            inf(nx)=0.
            f(nx)=zintp(nx,j)
            else
            factxm=(kdint(nx-1,j)+kdint(nx,j))/2.d0*(dt/2.)/dx**2
            factyp=(kdint(nx,j+1)+kdint(nx,j))/2.d0*(dt/2.)/dy**2
            factym=(kdint(nx,j-1)+kdint(nx,j))/2.d0*(dt/2.)/dy**2
            diag(nx)=1.d0+factxm
            inf(nx)=-factxm
            f(nx)=zintp(nx,j)+factyp*zintp(nx,j+1)-(factyp+factym)*zintp(nx,j)+factym*zintp(nx,j-1)
            endif
        */
        if true {
            diag[nx - 1] = 1.0;
            inf[nx - 1] = 0.0;
            f[nx - 1] = zintp[aij(nx - 1, j)];
        } else {
            // reflective boundary
            factxm = (kdint[aij(nx - 2, j)] + kdint[aij(nx - 1, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factyp =
                (kdint[aij(nx - 1, j + 1)] + kdint[aij(nx - 1, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            factym =
                (kdint[aij(nx - 1, j - 1)] + kdint[aij(nx - 1, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            diag[nx - 1] = 1.0 + factxm;
            inf[nx - 1] = -factxm;
            f[nx - 1] = zintp[aij(nx - 1, j)] + factyp * zintp[aij(nx - 1, j + 1)]
                - (factyp + factym) * zintp[aij(nx - 1, j)]
                + factym * zintp[aij(nx - 1, j - 1)];
        }
        /*
          call tridag (inf,diag,sup,f,res,nx)
            do i=1,nx
            zint(i,j)=res(i)
            enddo
        */
        tridag(&inf, &diag, &sup, &f, &mut res, nx);
        for i in 0..nx {
            zint[aij(i, j)] = res[i];
        }
        /*
          enddo
          deallocate (f,diag,sup,inf,res)
        */
    }

    /*
      ! second pass along y-axis

      allocate (f(ny),diag(ny),sup(ny),inf(ny),res(ny))
      f=0.d0
      diag=0.d0
      sup=0.d0
      inf=0.d0
      res=0.d0
      do i=2,nx-1
    */
    f = vec![0.0; ny];
    diag = vec![0.0; ny];
    sup = vec![0.0; ny];
    inf = vec![0.0; ny];
    res = vec![0.0; ny];
    for i in 1..nx - 1 {
        /*
            do j=2,ny-1
            factxp=(kdint(i+1,j)+kdint(i,j))/2.d0*(dt/2.)/dx**2
            factxm=(kdint(i-1,j)+kdint(i,j))/2.d0*(dt/2.)/dx**2
            factyp=(kdint(i,j+1)+kdint(i,j))/2.d0*(dt/2.)/dy**2
            factym=(kdint(i,j-1)+kdint(i,j))/2.d0*(dt/2.)/dy**2
            diag(j)=1.d0+factyp+factym
            sup(j)=-factyp
            inf(j)=-factym
            f(j)=zint(i,j)+factxp*zint(i+1,j)-(factxp+factxm)*zint(i,j)+factxm*zint(i-1,j)
            enddo
        */
        for j in 1..ny - 1 {
            factxp = (kdint[aij(i + 1, j)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factxm = (kdint[aij(i - 1, j)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factyp = (kdint[aij(i, j + 1)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            factym = (kdint[aij(i, j - 1)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dy * dy);
            diag[j] = 1.0 + factyp + factym;
            sup[j] = -factyp;
            inf[j] = -factym;
            f[j] = zint[aij(i, j)] + factxp * zint[aij(i + 1, j)]
                - (factxp + factxm) * zint[aij(i, j)]
                + factxm * zint[aij(i - 1, j)];
        }
        /*
        ! bottom bc
            if (cbc(1:1).eq.'1') then
            diag(1)=1.
            sup(1)=0.
            f(1)=zint(i,1)
            else
            factxp=(kdint(i+1,1)+kdint(i,j))/2.d0*(dt/2.)/dx**2
            factxm=(kdint(i-1,1)+kdint(i,1))/2.d0*(dt/2.)/dx**2
            factyp=(kdint(i,2)+kdint(i,1))/2.d0*(dt/2.)/dy**2
            diag(1)=1.d0+factyp
            sup(1)=-factyp
            f(1)=zint(i,1)+factxp*zint(i+1,1)-(factxp+factxm)*zint(i,1)+factxm*zint(i-1,1)
            endif
        */
        if true {
            diag[0] = 1.0;
            sup[0] = 0.0;
            f[0] = zint[aij(i, 0)];
        } else {
            // reflective boundary
            // TODO: Check whether this j was actually supposed to be a 0 in the original
            // (probably).
            // factxp = (kdint[aij(i+1, 0)] + kdint[aij(i, j)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factxp = (kdint[aij(i + 1, 0)] + kdint[aij(i, 0)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factxm = (kdint[aij(i - 1, 0)] + kdint[aij(i, 0)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factyp = (kdint[aij(i, 1)] + kdint[aij(i, 0)]) / 2.0 * (dt / 2.0) / (dy * dy);
            diag[0] = 1.0 + factyp;
            sup[0] = -factyp;
            f[0] = zint[aij(i, 0)] + factxp * zint[aij(i + 1, 0)]
                - (factxp + factxm) * zint[aij(i, 0)]
                + factxm * zint[aij(i - 1, 0)];
        }
        /*
        ! top bc
            if (cbc(3:3).eq.'1') then
            diag(ny)=1.
            inf(ny)=0.
            f(ny)=zint(i,ny)
            else
            factxp=(kdint(i+1,ny)+kdint(i,ny))/2.d0*(dt/2.)/dx**2
            factxm=(kdint(i-1,ny)+kdint(i,ny))/2.d0*(dt/2.)/dx**2
            factym=(kdint(i,ny-1)+kdint(i,ny))/2.d0*(dt/2.)/dy**2
            diag(ny)=1.d0+factym
            inf(ny)=-factym
            f(ny)=zint(i,ny)+factxp*zint(i+1,ny)-(factxp+factxm)*zint(i,ny)+factxm*zint(i-1,ny)
            endif
        */
        if true {
            diag[ny - 1] = 1.0;
            inf[ny - 1] = 0.0;
            f[ny - 1] = zint[aij(i, ny - 1)];
        } else {
            // reflective boundary
            factxp =
                (kdint[aij(i + 1, ny - 1)] + kdint[aij(i, ny - 1)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factxm =
                (kdint[aij(i - 1, ny - 1)] + kdint[aij(i, ny - 1)]) / 2.0 * (dt / 2.0) / (dx * dx);
            factym = (kdint[aij(i, ny - 2)] + kdint[aij(i, ny - 1)]) / 2.0 * (dt / 2.0) / (dy * dy);
            diag[ny - 1] = 1.0 + factym;
            inf[ny - 1] = -factym;
            f[ny - 1] = zint[aij(i, ny - 1)] + factxp * zint[aij(i + 1, ny - 1)]
                - (factxp + factxm) * zint[aij(i, ny - 1)]
                + factxm * zint[aij(i - 1, ny - 1)];
        }
        /*
          call tridag (inf,diag,sup,f,res,ny)
            do j=1,ny
            zintp(i,j)=res(j)
            enddo
        */
        tridag(&inf, &diag, &sup, &f, &mut res, ny);
        for j in 0..ny {
            zintp[aij(i, j)] = res[j];
        }
        /*
          enddo
          deallocate (f,diag,sup,inf,res)
        */
    }
    /*
      ! stores result in 1D array

      do j=1,ny
        do i=1,nx
          ij=(j-1)*nx+i
          etot(ij)=etot(ij)+h(ij)-zintp(i,j)
          erate(ij)=erate(ij)+(h(ij)-zintp(i,j))/dt
          h(ij)=zintp(i,j)
        enddo
      enddo

      b=min(h,b)
    */
    for j in 0..ny {
        for i in 0..nx {
            ij = aij(i, j);
            // FIXME: Track total erosion and erosion rate.
            h[ij] = zintp[ij] as Alt;
        }
    }

    b.par_iter_mut().zip(h).for_each(|(b, h)| {
        *b = h.min(*b);
    });
    /*
      deallocate (zint,kdint,zintp)

      return

    end subroutine Diffusion
    */
}

/*
!----------

! subroutine to solve a tri-diagonal system of equations (from Numerical Recipes)

      SUBROUTINE tridag(a,b,c,r,u,n)

      implicit none

      INTEGER n
      double precision a(n),b(n),c(n),r(n),u(n)
*/
pub fn tridag(a: &[f64], b: &[f64], c: &[f64], r: &[f64], u: &mut [f64], n: usize) {
    /*
          INTEGER j
          double precision bet
          double precision,dimension(:),allocatable::gam

          allocate (gam(n))

          if(b(1).eq.0.d0) stop 'in tridag'
    */
    let mut bet: f64;
    let mut gam: Vec<f64>;

    gam = vec![Default::default(); n];

    assert!(b[0] != 0.0);
    /*

    ! first pass

    bet=b(1)
    u(1)=r(1)/bet
    do 11 j=2,n
      gam(j)=c(j-1)/bet
      bet=b(j)-a(j)*gam(j)
      if(bet.eq.0.) then
        print*,'tridag failed'
        stop
      endif
      u(j)=(r(j)-a(j)*u(j-1))/bet
      11    continue
    */
    bet = b[0];
    u[0] = r[0] / bet;
    for j in 1..n {
        gam[j] = c[j - 1] / bet;
        bet = b[j] - a[j] * gam[j];
        assert!(bet != 0.0);
        // Round 0: u[0] = r[0] / b[0]
        //               = r'[0] / b'[0]
        // Round j: u[j] = (r[j] - a[j] * u'[j - 1]) / b'[j]
        //               = (r[j] - a[j] * r'[j - 1] / b'[j - 1]) / b'[j]
        //               = (r[j] - (a[j] / b'[j - 1]) * r'[j - 1]) / b'[j]
        //               = (r[j] - w[j] * r'[j - 1]) / b'[j]
        //               = r'[j] / b'[j]
        u[j] = (r[j] - a[j] * u[j - 1]) / bet;
    }
    /*
      ! second pass

      do 12 j=n-1,1,-1
        u(j)=u(j)-gam(j+1)*u(j+1)
        12    continue
    */
    for j in (0..n - 1).rev() {
        u[j] = u[j] - gam[j + 1] * u[j + 1];
    }
    /*
        deallocate (gam)

        return

        END
    */
}
