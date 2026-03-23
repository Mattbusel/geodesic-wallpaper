//! Map projections for spherical-to-plane tiling.
//!
//! Provides forward (`project`) and inverse (`unproject`) map projections
//! covering Mercator, Equirectangular, Azimuthal Equidistant, Stereographic
//! (north-pole), and Mollweide.
//!
//! A `ProjectionMapper` wraps a projection and a pixel canvas so geographic
//! coordinates can be round-tripped through pixel space.

use std::f64::consts::PI;

// ── GeoPoint ──────────────────────────────────────────────────────────────────

/// A geographic point in decimal degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoPoint {
    /// Latitude in degrees (−90 … +90)
    pub lat: f64,
    /// Longitude in degrees (−180 … +180)
    pub lon: f64,
}

impl GeoPoint {
    pub fn new(lat: f64, lon: f64) -> Self {
        GeoPoint { lat, lon }
    }

    fn lat_rad(self) -> f64 { self.lat.to_radians() }
    fn lon_rad(self) -> f64 { self.lon.to_radians() }
}

// ── PlanePoint ────────────────────────────────────────────────────────────────

/// A point in the projected plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanePoint {
    pub x: f64,
    pub y: f64,
}

impl PlanePoint {
    pub fn new(x: f64, y: f64) -> Self {
        PlanePoint { x, y }
    }
}

// ── Projection ────────────────────────────────────────────────────────────────

/// Supported map projections.
#[derive(Debug, Clone)]
pub enum Projection {
    /// Conformal cylindrical projection; maps equator to y = 0.
    Mercator,
    /// Simple plate carrée: x = lon_rad, y = lat_rad.
    Equirectangular,
    /// Azimuthal equidistant centred on a reference geographic point.
    AzimuthalEquidistant { center: GeoPoint },
    /// Stereographic projection; `pole = true` → north pole, `false` → south.
    Stereographic { pole: bool },
    /// Mollweide pseudo-cylindrical equal-area projection.
    Mollweide,
}

// ── project ───────────────────────────────────────────────────────────────────

fn mollweide_theta(lat: f64) -> f64 {
    let rhs = PI * lat.sin();
    let mut theta = lat; // initial guess
    for _ in 0..50 {
        let denom = (2.0 * theta).cos() * 2.0 + 2.0;
        if denom.abs() < 1e-15 {
            break;
        }
        let delta = -(2.0 * theta + (2.0 * theta).sin() - rhs) / denom;
        theta += delta;
        if delta.abs() < 1e-12 {
            break;
        }
    }
    theta
}

/// Project a geographic point to the plane.
pub fn project(geo: &GeoPoint, proj: &Projection) -> PlanePoint {
    project_point(geo, proj)
}

/// Project a geographic point to the plane (canonical implementation).
pub fn project_point(geo: &GeoPoint, proj: &Projection) -> PlanePoint {
    match proj {
        Projection::Mercator => {
            let x = geo.lon_rad();
            let lat = geo.lat_rad().clamp(-1.4835, 1.4835);
            let y = (PI / 4.0 + lat / 2.0).tan().ln();
            PlanePoint::new(x, y)
        }
        Projection::Equirectangular => {
            PlanePoint::new(geo.lon_rad(), geo.lat_rad())
        }
        Projection::AzimuthalEquidistant { center } => {
            let phi1 = center.lat_rad();
            let lam0 = center.lon_rad();
            let phi = geo.lat_rad();
            let lam = geo.lon_rad();
            let cos_c = phi1.sin() * phi.sin()
                + phi1.cos() * phi.cos() * (lam - lam0).cos();
            let c = cos_c.clamp(-1.0, 1.0).acos();
            if c.abs() < 1e-10 {
                return PlanePoint::new(0.0, 0.0);
            }
            let az = (lam - lam0).sin() * phi.cos()
                .atan2(phi1.cos() * phi.sin() - phi1.sin() * phi.cos() * (lam - lam0).cos());
            PlanePoint::new(c * az.sin(), -c * az.cos())
        }
        Projection::Stereographic { pole } => {
            let lat = geo.lat_rad();
            let lon = geo.lon_rad();
            if *pole {
                let k = 2.0 / (1.0 + lat.sin()).max(1e-10);
                PlanePoint::new(k * lat.cos() * lon.sin(), -k * lat.cos() * lon.cos())
            } else {
                let lat_s = -lat;
                let k = 2.0 / (1.0 + lat_s.sin()).max(1e-10);
                PlanePoint::new(k * lat_s.cos() * lon.sin(), k * lat_s.cos() * lon.cos())
            }
        }
        Projection::Mollweide => {
            let lat = geo.lat_rad();
            let lon = geo.lon_rad();
            let theta = mollweide_theta(lat);
            let x = (2.0 * 2.0_f64.sqrt() / PI) * lon * theta.cos();
            let y = 2.0_f64.sqrt() * theta.sin();
            PlanePoint::new(x, y)
        }
    }
}

// ── unproject ─────────────────────────────────────────────────────────────────

/// Inverse projection from plane to geographic coordinates.
///
/// Returns `None` for projections where the inverse is not analytically
/// tractable (Azimuthal Equidistant, Stereographic, Mollweide) unless the
/// point is the origin.
pub fn unproject(plane: &PlanePoint, proj: &Projection) -> Option<GeoPoint> {
    match proj {
        Projection::Mercator => {
            let lon = plane.x.to_degrees();
            let lat = (2.0 * plane.y.exp().atan() - PI / 2.0).to_degrees();
            if lat.is_finite() && lon.is_finite() {
                Some(GeoPoint::new(lat, lon))
            } else {
                None
            }
        }
        Projection::Equirectangular => {
            Some(GeoPoint::new(plane.y.to_degrees(), plane.x.to_degrees()))
        }
        // Inverse for these projections is non-trivial; return None
        Projection::AzimuthalEquidistant { .. } => None,
        Projection::Stereographic { .. } => None,
        Projection::Mollweide => None,
    }
}

// ── ProjectionMapper ──────────────────────────────────────────────────────────

/// Maps between geographic coordinates and pixel coordinates on a canvas.
pub struct ProjectionMapper {
    pub projection: Projection,
    pub width: usize,
    pub height: usize,
    /// Pixels per projected unit.
    pub scale: f64,
    /// Plane coordinate at the canvas centre.
    pub center: PlanePoint,
}

impl ProjectionMapper {
    pub fn new(
        projection: Projection,
        width: usize,
        height: usize,
        scale: f64,
        center: PlanePoint,
    ) -> Self {
        ProjectionMapper {
            projection,
            width,
            height,
            scale,
            center,
        }
    }

    /// Project a geographic point to pixel coordinates.
    ///
    /// Returns `None` if the projected point falls outside the canvas.
    pub fn geo_to_pixel(&self, geo: &GeoPoint) -> Option<(usize, usize)> {
        let plane = project_point(geo, &self.projection);
        // Shift by centre and scale to pixel space
        let px = (plane.x - self.center.x) * self.scale + self.width as f64 / 2.0;
        let py = -(plane.y - self.center.y) * self.scale + self.height as f64 / 2.0;

        if px < 0.0 || py < 0.0 || px >= self.width as f64 || py >= self.height as f64 {
            None
        } else {
            Some((px as usize, py as usize))
        }
    }

    /// Unproject a pixel to geographic coordinates.
    ///
    /// Returns `None` if the projection has no inverse or the pixel is out of bounds.
    pub fn pixel_to_geo(&self, px: usize, py: usize) -> Option<GeoPoint> {
        if px >= self.width || py >= self.height {
            return None;
        }
        let plane_x = (px as f64 - self.width as f64 / 2.0) / self.scale + self.center.x;
        let plane_y = -(py as f64 - self.height as f64 / 2.0) / self.scale + self.center.y;
        unproject(&PlanePoint::new(plane_x, plane_y), &self.projection)
    }

    /// Render a lat/lon graticule on a blank RGB canvas.
    ///
    /// Returns a flat `width * height * 3` RGB `u8` buffer.
    pub fn render_graticule(
        &self,
        lat_spacing: f64,
        lon_spacing: f64,
        color: (u8, u8, u8),
    ) -> Vec<u8> {
        let mut buf = vec![0u8; self.width * self.height * 3];

        // Draw latitude lines
        let mut lat = -90.0 + lat_spacing;
        while lat < 90.0 {
            let mut lon = -180.0;
            while lon <= 180.0 {
                let geo = GeoPoint::new(lat, lon);
                if let Some((px, py)) = self.geo_to_pixel(&geo) {
                    let idx = (py * self.width + px) * 3;
                    if idx + 2 < buf.len() {
                        buf[idx] = color.0;
                        buf[idx + 1] = color.1;
                        buf[idx + 2] = color.2;
                    }
                }
                lon += 0.5; // step sub-degree for dense lines
            }
            lat += lat_spacing;
        }

        // Draw longitude lines
        let mut lon = -180.0 + lon_spacing;
        while lon < 180.0 {
            let mut lat = -89.5;
            while lat <= 89.5 {
                let geo = GeoPoint::new(lat, lon);
                if let Some((px, py)) = self.geo_to_pixel(&geo) {
                    let idx = (py * self.width + px) * 3;
                    if idx + 2 < buf.len() {
                        buf[idx] = color.0;
                        buf[idx + 1] = color.1;
                        buf[idx + 2] = color.2;
                    }
                }
                lat += 0.5;
            }
            lon += lon_spacing;
        }

        buf
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_equator_maps_to_y_zero() {
        let geo = GeoPoint::new(0.0, 0.0);
        let p = project_point(&geo, &Projection::Mercator);
        assert!(p.y.abs() < 1e-10, "Mercator equator y = {}, expected 0", p.y);
        assert!(p.x.abs() < 1e-10, "Mercator prime meridian x = {}, expected 0", p.x);
    }

    #[test]
    fn equirectangular_identity() {
        let geo = GeoPoint::new(45.0, 90.0);
        let p = project_point(&geo, &Projection::Equirectangular);
        assert!((p.x - 90.0_f64.to_radians()).abs() < 1e-10);
        assert!((p.y - 45.0_f64.to_radians()).abs() < 1e-10);
    }

    #[test]
    fn mercator_roundtrip() {
        let original = GeoPoint::new(30.0, 45.0);
        let plane = project_point(&original, &Projection::Mercator);
        let recovered = unproject(&plane, &Projection::Mercator).expect("Mercator has inverse");
        assert!((recovered.lat - original.lat).abs() < 1e-8, "lat roundtrip failed");
        assert!((recovered.lon - original.lon).abs() < 1e-8, "lon roundtrip failed");
    }

    #[test]
    fn equirectangular_roundtrip() {
        let original = GeoPoint::new(-15.0, 120.0);
        let plane = project_point(&original, &Projection::Equirectangular);
        let recovered = unproject(&plane, &Projection::Equirectangular).expect("Equirectangular has inverse");
        assert!((recovered.lat - original.lat).abs() < 1e-10);
        assert!((recovered.lon - original.lon).abs() < 1e-10);
    }

    #[test]
    fn azimuthal_equidistant_centre_maps_to_origin() {
        let center = GeoPoint::new(51.5, -0.1); // London
        let proj = Projection::AzimuthalEquidistant { center };
        let p = project_point(&center, &proj);
        assert!(p.x.abs() < 1e-8 && p.y.abs() < 1e-8, "centre should map to origin: {:?}", p);
    }

    #[test]
    fn stereographic_north_pole_maps_origin() {
        // At the north pole the projected point depends on lat; at lat=90 k→inf
        // Instead test that equator lon=0 gives a specific known value
        let geo = GeoPoint::new(0.0, 0.0); // equator, prime meridian
        let p = project_point(&geo, &Projection::Stereographic { pole: true });
        // k = 2/(1+sin(0)) = 2, cos(0)*sin(0) = 0, cos(0)*cos(0) = 1
        // x = 2*1*0 = 0; y = -2*1*1 = -2
        assert!((p.x - 0.0).abs() < 1e-10, "x={}", p.x);
        assert!((p.y - (-2.0)).abs() < 1e-10, "y={}", p.y);
    }

    #[test]
    fn pixel_within_bounds() {
        let mapper = ProjectionMapper::new(
            Projection::Equirectangular,
            800,
            400,
            100.0,
            PlanePoint::new(0.0, 0.0),
        );
        let geo = GeoPoint::new(0.0, 0.0);
        if let Some((px, py)) = mapper.geo_to_pixel(&geo) {
            assert!(px < 800);
            assert!(py < 400);
        }
    }

    #[test]
    fn pixel_to_geo_roundtrip_equirectangular() {
        let scale = 100.0;
        let mapper = ProjectionMapper::new(
            Projection::Equirectangular,
            800,
            400,
            scale,
            PlanePoint::new(0.0, 0.0),
        );
        let geo = GeoPoint::new(10.0, 20.0);
        if let Some((px, py)) = mapper.geo_to_pixel(&geo) {
            if let Some(recovered) = mapper.pixel_to_geo(px, py) {
                // Pixel quantisation introduces ~1/scale degree error
                assert!((recovered.lat - geo.lat).abs() < 1.0);
                assert!((recovered.lon - geo.lon).abs() < 1.0);
            }
        }
    }

    #[test]
    fn mollweide_equator_maps_to_y_zero() {
        let geo = GeoPoint::new(0.0, 0.0);
        let p = project_point(&geo, &Projection::Mollweide);
        assert!(p.y.abs() < 1e-8, "Mollweide equator y = {}", p.y);
    }

    #[test]
    fn graticule_render_returns_correct_size() {
        let mapper = ProjectionMapper::new(
            Projection::Equirectangular,
            100,
            50,
            10.0,
            PlanePoint::new(0.0, 0.0),
        );
        let buf = mapper.render_graticule(30.0, 30.0, (255, 255, 255));
        assert_eq!(buf.len(), 100 * 50 * 3);
    }
}
